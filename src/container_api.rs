use crate::types::{
    Annotation, Char, Curve, Edge, Hyperlink, ImageObject, Line, Orientation,
    Page, PdfDocument, RectObject,
};
use crate::{Error, Result};
use serde::Serialize;
use serde_json::{json, Map, Number, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

const CACHED_PROPERTIES: &[&str] = &["_rect_edges", "_curve_edges", "_edges", "_objects"];
const OBJECT_TYPES: &[&str] = &[
    "char",
    "line",
    "rect",
    "curve",
    "image",
    "annot",
    "hyperlink",
    "textboxhorizontal",
    "textboxvertical",
    "textlinehorizontal",
    "textlinevertical",
];

fn singular_to_plural(name: &str) -> String {
    match name {
        "char" => "chars".to_string(),
        "line" => "lines".to_string(),
        "rect" => "rects".to_string(),
        "curve" => "curves".to_string(),
        "image" => "images".to_string(),
        "annot" => "annots".to_string(),
        "hyperlink" => "hyperlinks".to_string(),
        "textboxhorizontal" => "textboxhorizontals".to_string(),
        "textboxvertical" => "textboxverticals".to_string(),
        "textlinehorizontal" => "textlinehorizontals".to_string(),
        "textlinevertical" => "textlineverticals".to_string(),
        other => format!("{other}s"),
    }
}

fn normalize_requested_types(object_types: Option<&[&str]>) -> Vec<String> {
    match object_types {
        Some(requested) => requested
            .iter()
            .map(|name| match *name {
                "chars" => "char".to_string(),
                "lines" => "line".to_string(),
                "rects" => "rect".to_string(),
                "curves" => "curve".to_string(),
                "images" => "image".to_string(),
                "annots" => "annot".to_string(),
                "hyperlinks" => "hyperlink".to_string(),
                "textboxhorizontals" => "textboxhorizontal".to_string(),
                "textboxverticals" => "textboxvertical".to_string(),
                "textlinehorizontals" => "textlinehorizontal".to_string(),
                "textlineverticals" => "textlinevertical".to_string(),
                other => other.to_string(),
            })
            .collect(),
        None => OBJECT_TYPES.iter().map(|name| (*name).to_string()).collect(),
    }
}

fn value_from_serialize<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

fn round_json(value: &mut Value, precision: Option<usize>) {
    match value {
        Value::Number(number) => {
            if let Some(precision) = precision {
                if let Some(float) = number.as_f64() {
                    let factor = 10f64.powi(precision as i32);
                    let rounded = (float * factor).round() / factor;
                    if let Some(new_number) = Number::from_f64(rounded) {
                        *number = new_number;
                    }
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                round_json(item, precision);
            }
        }
        Value::Object(map) => {
            for item in map.values_mut() {
                round_json(item, precision);
            }
        }
        _ => {}
    }
}

fn filter_json_attrs(value: &mut Value, include: Option<&BTreeSet<String>>, exclude: Option<&BTreeSet<String>>) {
    match value {
        Value::Array(items) => {
            for item in items {
                filter_json_attrs(item, include, exclude);
            }
        }
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                let include_match = include.map(|set| set.contains(&key)).unwrap_or(true);
                let exclude_match = exclude.map(|set| set.contains(&key)).unwrap_or(false);
                if !include_match || exclude_match {
                    map.remove(&key);
                }
            }
            for item in map.values_mut() {
                filter_json_attrs(item, include, exclude);
            }
        }
        _ => {}
    }
}

fn json_to_string(value: &Value, indent: Option<usize>) -> Result<String> {
    if let Some(indent) = indent {
        let mut buf = Vec::new();
        let indent_vec = vec![b' '; indent.max(1)];
        let formatter = serde_json::ser::PrettyFormatter::with_indent(&indent_vec);
        let mut serializer = serde_json::Serializer::with_formatter(&mut buf, formatter);
        value.serialize(&mut serializer)?;
        Ok(String::from_utf8(buf).map_err(|err| Error::Message(err.to_string()))?)
    } else {
        Ok(serde_json::to_string(value)?)
    }
}

fn should_emit_object_array(object_type: &str, value: &Value) -> bool {
    match value {
        Value::Array(items) => !items.is_empty() || object_type == "annot",
        _ => false,
    }
}

fn flatten_value(prefix: Option<&str>, value: &Value, out: &mut BTreeMap<String, String>) {
    match value {
        Value::Object(map) => {
            for (key, item) in map {
                let next = match prefix {
                    Some(prefix) if !prefix.is_empty() => format!("{prefix}.{key}"),
                    _ => key.clone(),
                };
                match item {
                    Value::Object(_) => flatten_value(Some(&next), item, out),
                    Value::Array(_) => {
                        out.insert(next, item.to_string());
                    }
                    Value::Null => {
                        out.insert(next, String::new());
                    }
                    Value::String(s) => {
                        out.insert(next, s.clone());
                    }
                    Value::Bool(b) => {
                        out.insert(next, if *b { "1".to_string() } else { "0".to_string() });
                    }
                    Value::Number(n) => {
                        out.insert(next, n.to_string());
                    }
                }
            }
        }
        _ => {
            let key = prefix.unwrap_or("value").to_string();
            out.insert(key, value.to_string());
        }
    }
}

fn write_or_return<W: Write>(mut stream: Option<W>, output: String) -> Result<Option<String>> {
    if let Some(writer) = stream.as_mut() {
        writer.write_all(output.as_bytes())?;
        Ok(None)
    } else {
        Ok(Some(output))
    }
}

fn page_object_array(page: &Page, object_type: &str) -> Value {
    match object_type {
        "char" => value_from_serialize(&page.chars),
        "line" => value_from_serialize(&page.lines),
        "rect" => value_from_serialize(&page.rects),
        "curve" => value_from_serialize(&page.curves),
        "image" => value_from_serialize(&page.images),
        "annot" => value_from_serialize(&page.annots),
        "hyperlink" => value_from_serialize(&page.hyperlinks),
        "textboxhorizontal" => value_from_serialize(&page.textboxhorizontals()),
        "textboxvertical" => value_from_serialize(&page.textboxverticals()),
        "textlinehorizontal" => value_from_serialize(&page.textlinehorizontals()),
        "textlinevertical" => value_from_serialize(&page.textlineverticals()),
        _ => Value::Array(Vec::new()),
    }
}

fn doc_object_array(doc: &PdfDocument, object_type: &str) -> Value {
    match object_type {
        "char" => value_from_serialize(&doc.chars()),
        "line" => value_from_serialize(&doc.lines()),
        "rect" => value_from_serialize(&doc.rects()),
        "curve" => value_from_serialize(&doc.curves()),
        "image" => value_from_serialize(&doc.images()),
        "annot" => value_from_serialize(&doc.annots()),
        "hyperlink" => value_from_serialize(&doc.hyperlinks()),
        "textboxhorizontal" => value_from_serialize(&doc.textboxhorizontals()),
        "textboxvertical" => value_from_serialize(&doc.textboxverticals()),
        "textlinehorizontal" => value_from_serialize(&doc.textlinehorizontals()),
        "textlinevertical" => value_from_serialize(&doc.textlineverticals()),
        _ => Value::Array(Vec::new()),
    }
}

fn page_object_rows(page: &Page, object_types: Option<&[&str]>, precision: Option<usize>, include_attrs: Option<&[&str]>, exclude_attrs: Option<&[&str]>) -> Vec<BTreeMap<String, String>> {
    let include = include_attrs.map(|attrs| attrs.iter().map(|item| (*item).to_string()).collect::<BTreeSet<_>>());
    let exclude = exclude_attrs.map(|attrs| attrs.iter().map(|item| (*item).to_string()).collect::<BTreeSet<_>>());

    let mut rows = Vec::new();
    for object_type in normalize_requested_types(object_types) {
        let mut value = page_object_array(page, &object_type);
        round_json(&mut value, precision);
        filter_json_attrs(&mut value, include.as_ref(), exclude.as_ref());
        if let Value::Array(items) = value {
            for item in items {
                let mut flat = BTreeMap::new();
                flatten_value(None, &item, &mut flat);
                rows.push(flat);
            }
        }
    }
    rows
}

fn document_object_rows(doc: &PdfDocument, object_types: Option<&[&str]>, precision: Option<usize>, include_attrs: Option<&[&str]>, exclude_attrs: Option<&[&str]>) -> Vec<BTreeMap<String, String>> {
    let include = include_attrs.map(|attrs| attrs.iter().map(|item| (*item).to_string()).collect::<BTreeSet<_>>());
    let exclude = exclude_attrs.map(|attrs| attrs.iter().map(|item| (*item).to_string()).collect::<BTreeSet<_>>());

    let mut rows = Vec::new();
    for object_type in normalize_requested_types(object_types) {
        let mut value = doc_object_array(doc, &object_type);
        round_json(&mut value, precision);
        filter_json_attrs(&mut value, include.as_ref(), exclude.as_ref());
        if let Value::Array(items) = value {
            for item in items {
                let mut flat = BTreeMap::new();
                flatten_value(None, &item, &mut flat);
                rows.push(flat);
            }
        }
    }
    rows
}

fn rows_to_csv(rows: &[BTreeMap<String, String>]) -> Result<String> {
    let mut headers = BTreeSet::new();
    for row in rows {
        for key in row.keys() {
            headers.insert(key.clone());
        }
    }
    let headers: Vec<String> = headers.into_iter().collect();

    let mut output = Vec::new();
    {
        let mut writer = csv::Writer::from_writer(&mut output);
        writer.write_record(headers.iter())?;
        for row in rows {
            let record = headers
                .iter()
                .map(|header| row.get(header).cloned().unwrap_or_default())
                .collect::<Vec<_>>();
            writer.write_record(record)?;
        }
        writer.flush()?;
    }
    Ok(String::from_utf8(output).map_err(|err| Error::Message(err.to_string()))?)
}

impl Page {
    pub fn cached_properties(&self) -> &'static [&'static str] {
        CACHED_PROPERTIES
    }

    pub fn close(&mut self) {}

    pub fn flush_cache(&mut self, _properties: Option<&[&str]>) {}

    pub fn rect_edges(&self) -> Vec<Edge> {
        self.edges()
            .into_iter()
            .filter(|edge| edge.object_type == "rect_edge")
            .collect()
    }

    pub fn curve_edges(&self) -> Vec<Edge> {
        self.edges()
            .into_iter()
            .filter(|edge| edge.object_type == "curve_edge")
            .collect()
    }

    pub fn horizontal_edges(&self) -> Vec<Edge> {
        self.edges()
            .into_iter()
            .filter(|edge| edge.orientation == Orientation::Horizontal)
            .collect()
    }

    pub fn vertical_edges(&self) -> Vec<Edge> {
        self.edges()
            .into_iter()
            .filter(|edge| edge.orientation == Orientation::Vertical)
            .collect()
    }

    pub fn objects(&self) -> BTreeMap<String, Value> {
        let mut out = BTreeMap::new();
        for object_type in OBJECT_TYPES {
            let value = page_object_array(self, object_type);
            if should_emit_object_array(object_type, &value) {
                out.insert((*object_type).to_string(), value);
            }
        }
        out
    }

    pub fn to_dict(&self, object_types: Option<&[&str]>) -> Value {
        let mut out = Map::new();
        out.insert("page_number".to_string(), json!(self.page_number));
        out.insert("initial_doctop".to_string(), json!(self.doctop_offset));
        out.insert("rotation".to_string(), json!(self.rotation));
        out.insert("cropbox".to_string(), json!([self.cropbox.x0, self.cropbox.top, self.cropbox.x1, self.cropbox.bottom]));
        out.insert("mediabox".to_string(), json!([self.mediabox.x0, self.mediabox.top, self.mediabox.x1, self.mediabox.bottom]));
        if let Some(trimbox) = self.trimbox {
            out.insert("trimbox".to_string(), json!([trimbox.x0, trimbox.top, trimbox.x1, trimbox.bottom]));
        }
        if let Some(bleedbox) = self.bleedbox {
            out.insert("bleedbox".to_string(), json!([bleedbox.x0, bleedbox.top, bleedbox.x1, bleedbox.bottom]));
        }
        if let Some(artbox) = self.artbox {
            out.insert("artbox".to_string(), json!([artbox.x0, artbox.top, artbox.x1, artbox.bottom]));
        }
        out.insert("bbox".to_string(), json!([self.bbox.x0, self.bbox.top, self.bbox.x1, self.bbox.bottom]));
        out.insert("width".to_string(), json!(self.width));
        out.insert("height".to_string(), json!(self.height));
        out.insert("is_original".to_string(), json!(self.is_original));
        if let Some(structure_tree) = &self.structure_tree {
            out.insert("structure_tree".to_string(), value_from_serialize(structure_tree));
        }

        for object_type in normalize_requested_types(object_types) {
            let value = page_object_array(self, &object_type);
            if should_emit_object_array(&object_type, &value) {
                out.insert(singular_to_plural(&object_type), value);
            }
        }

        Value::Object(out)
    }

    pub fn to_json<W: Write>(
        &self,
        stream: Option<W>,
        object_types: Option<&[&str]>,
        include_attrs: Option<&[&str]>,
        exclude_attrs: Option<&[&str]>,
        precision: Option<usize>,
        indent: Option<usize>,
    ) -> Result<Option<String>> {
        let mut value = self.to_dict(object_types);
        round_json(&mut value, precision);
        let include = include_attrs.map(|attrs| attrs.iter().map(|item| (*item).to_string()).collect::<BTreeSet<_>>());
        let exclude = exclude_attrs.map(|attrs| attrs.iter().map(|item| (*item).to_string()).collect::<BTreeSet<_>>());
        filter_json_attrs(&mut value, include.as_ref(), exclude.as_ref());
        let output = json_to_string(&value, indent)?;
        write_or_return(stream, output)
    }

    pub fn to_csv<W: Write>(
        &self,
        stream: Option<W>,
        object_types: Option<&[&str]>,
        precision: Option<usize>,
        include_attrs: Option<&[&str]>,
        exclude_attrs: Option<&[&str]>,
    ) -> Result<Option<String>> {
        let rows = page_object_rows(self, object_types, precision, include_attrs, exclude_attrs);
        let output = rows_to_csv(&rows)?;
        write_or_return(stream, output)
    }
}

impl PdfDocument {
    pub fn cached_properties(&self) -> &'static [&'static str] {
        CACHED_PROPERTIES
    }

    pub fn close(&mut self) {}

    pub fn flush_cache(&mut self, _properties: Option<&[&str]>) {}

    pub fn chars(&self) -> Vec<Char> {
        self.pages.iter().flat_map(|page| page.chars.clone()).collect()
    }

    pub fn lines(&self) -> Vec<Line> {
        self.pages.iter().flat_map(|page| page.lines.clone()).collect()
    }

    pub fn rects(&self) -> Vec<RectObject> {
        self.pages.iter().flat_map(|page| page.rects.clone()).collect()
    }

    pub fn curves(&self) -> Vec<Curve> {
        self.pages.iter().flat_map(|page| page.curves.clone()).collect()
    }

    pub fn images(&self) -> Vec<ImageObject> {
        self.pages.iter().flat_map(|page| page.images.clone()).collect()
    }

    pub fn annots(&self) -> Vec<Annotation> {
        self.pages.iter().flat_map(|page| page.annots.clone()).collect()
    }

    pub fn hyperlinks(&self) -> Vec<Hyperlink> {
        self.pages.iter().flat_map(|page| page.hyperlinks.clone()).collect()
    }

    pub fn rect_edges(&self) -> Vec<Edge> {
        self.pages.iter().flat_map(|page| page.rect_edges()).collect()
    }

    pub fn curve_edges(&self) -> Vec<Edge> {
        self.pages.iter().flat_map(|page| page.curve_edges()).collect()
    }

    pub fn edges(&self) -> Vec<Edge> {
        self.pages.iter().flat_map(|page| page.edges()).collect()
    }

    pub fn horizontal_edges(&self) -> Vec<Edge> {
        self.pages.iter().flat_map(|page| page.horizontal_edges()).collect()
    }

    pub fn vertical_edges(&self) -> Vec<Edge> {
        self.pages.iter().flat_map(|page| page.vertical_edges()).collect()
    }

    pub fn objects(&self) -> BTreeMap<String, Value> {
        let mut out = BTreeMap::new();
        for object_type in OBJECT_TYPES {
            let value = doc_object_array(self, object_type);
            if should_emit_object_array(object_type, &value) {
                out.insert((*object_type).to_string(), value);
            }
        }
        out
    }

    pub fn to_dict(&self, object_types: Option<&[&str]>) -> Value {
        let pages = self
            .pages
            .iter()
            .map(|page| page.to_dict(object_types))
            .collect::<Vec<_>>();
        let mut out = Map::new();
        out.insert("metadata".to_string(), value_from_serialize(&self.metadata));
        out.insert("pages".to_string(), Value::Array(pages));
        if let Some(structure_tree) = &self.structure_tree {
            out.insert("structure_tree".to_string(), value_from_serialize(structure_tree));
        }
        Value::Object(out)
    }

    pub fn to_json<W: Write>(
        &self,
        stream: Option<W>,
        object_types: Option<&[&str]>,
        include_attrs: Option<&[&str]>,
        exclude_attrs: Option<&[&str]>,
        precision: Option<usize>,
        indent: Option<usize>,
    ) -> Result<Option<String>> {
        let mut value = self.to_dict(object_types);
        round_json(&mut value, precision);
        let include = include_attrs.map(|attrs| attrs.iter().map(|item| (*item).to_string()).collect::<BTreeSet<_>>());
        let exclude = exclude_attrs.map(|attrs| attrs.iter().map(|item| (*item).to_string()).collect::<BTreeSet<_>>());
        filter_json_attrs(&mut value, include.as_ref(), exclude.as_ref());
        let output = json_to_string(&value, indent)?;
        write_or_return(stream, output)
    }

    pub fn to_csv<W: Write>(
        &self,
        stream: Option<W>,
        object_types: Option<&[&str]>,
        precision: Option<usize>,
        include_attrs: Option<&[&str]>,
        exclude_attrs: Option<&[&str]>,
    ) -> Result<Option<String>> {
        let rows = document_object_rows(self, object_types, precision, include_attrs, exclude_attrs);
        let output = rows_to_csv(&rows)?;
        write_or_return(stream, output)
    }
}
