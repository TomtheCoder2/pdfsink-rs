use crate::text::{extract_text_lines, extract_words};
use crate::types::{BBox, Char, Direction, LayoutObject, Page, PageLayout, PdfDocument, StructureElement};
use std::collections::BTreeMap;

fn layout_object_from_bbox(
    object_type: &str,
    page_number: usize,
    bbox: BBox,
    text: Option<String>,
    direction: Option<Direction>,
    upright: Option<bool>,
    children: Vec<LayoutObject>,
) -> LayoutObject {
    LayoutObject {
        object_type: object_type.to_string(),
        page_number,
        x0: bbox.x0,
        top: bbox.top,
        x1: bbox.x1,
        bottom: bbox.bottom,
        width: bbox.width(),
        height: bbox.height(),
        text,
        direction,
        upright,
        children,
    }
}

fn merge_layout_children(children: &[LayoutObject]) -> Option<BBox> {
    let first = children.first()?;
    let mut bbox = first.bbox();
    for child in children.iter().skip(1) {
        let cb = child.bbox();
        bbox = BBox::new(
            bbox.x0.min(cb.x0),
            bbox.top.min(cb.top),
            bbox.x1.max(cb.x1),
            bbox.bottom.max(cb.bottom),
        );
    }
    Some(bbox)
}

fn build_textline_objects(page: &Page, chars: &[Char], vertical: bool) -> Vec<LayoutObject> {
    if chars.is_empty() {
        return Vec::new();
    }

    let mut options = page.default_text_options();
    if vertical {
        options.line_dir = Direction::Ltr;
        options.char_dir = Direction::Ttb;
        options.line_dir_rotated = Some(Direction::Ltr);
        options.char_dir_rotated = Some(Direction::Ttb);
    }

    extract_text_lines(chars, &options, false, true)
        .into_iter()
        .map(|line| {
            let bbox = BBox::new(line.x0, line.top, line.x1, line.bottom);
            layout_object_from_bbox(
                if vertical { "textlinevertical" } else { "textlinehorizontal" },
                page.page_number,
                bbox,
                Some(line.text),
                Some(if vertical { Direction::Ttb } else { Direction::Ltr }),
                Some(!vertical),
                Vec::new(),
            )
        })
        .collect()
}

fn build_textbox_objects(lines: &[LayoutObject], vertical: bool) -> Vec<LayoutObject> {
    if lines.is_empty() {
        return Vec::new();
    }

    let mut sorted = lines.to_vec();
    if vertical {
        sorted.sort_by(|a, b| a.x0.total_cmp(&b.x0).then_with(|| a.top.total_cmp(&b.top)));
    } else {
        sorted.sort_by(|a, b| a.top.total_cmp(&b.top).then_with(|| a.x0.total_cmp(&b.x0)));
    }

    let mut groups: Vec<Vec<LayoutObject>> = Vec::new();
    for line in sorted {
        let should_split = groups
            .last()
            .and_then(|group| group.last())
            .map(|prev| {
                let gap = if vertical {
                    line.x0 - prev.x1
                } else {
                    line.top - prev.bottom
                };
                let overlap = if vertical {
                    !(line.bottom < prev.top || line.top > prev.bottom)
                } else {
                    !(line.x1 < prev.x0 || line.x0 > prev.x1)
                };
                gap > 12.0 && !overlap
            })
            .unwrap_or(false);

        if should_split {
            groups.push(vec![line]);
        } else if let Some(group) = groups.last_mut() {
            group.push(line);
        } else {
            groups.push(vec![line]);
        }
    }

    groups
        .into_iter()
        .filter_map(|children| {
            let bbox = merge_layout_children(&children)?;
            let text = children
                .iter()
                .filter_map(|child| child.text.clone())
                .collect::<Vec<_>>()
                .join("\n");
            Some(layout_object_from_bbox(
                if vertical { "textboxvertical" } else { "textboxhorizontal" },
                children[0].page_number,
                bbox,
                Some(text),
                Some(if vertical { Direction::Ttb } else { Direction::Ltr }),
                Some(!vertical),
                children,
            ))
        })
        .collect()
}

impl Page {
    pub fn pages(&self) -> Vec<Page> {
        vec![self.clone()]
    }

    pub fn point2coord(&self, pt: (f64, f64)) -> (f64, f64) {
        (pt.0, self.height - pt.1)
    }

    pub fn textlinehorizontals(&self) -> Vec<LayoutObject> {
        let chars: Vec<Char> = self.chars.iter().filter(|ch| ch.upright).cloned().collect();
        build_textline_objects(self, &chars, false)
    }

    pub fn textlineverticals(&self) -> Vec<LayoutObject> {
        let chars: Vec<Char> = self.chars.iter().filter(|ch| !ch.upright).cloned().collect();
        build_textline_objects(self, &chars, true)
    }

    pub fn textboxhorizontals(&self) -> Vec<LayoutObject> {
        build_textbox_objects(&self.textlinehorizontals(), false)
    }

    pub fn textboxverticals(&self) -> Vec<LayoutObject> {
        build_textbox_objects(&self.textlineverticals(), true)
    }

    pub fn layout(&self) -> PageLayout {
        let mut objects = Vec::new();
        objects.extend(self.textboxhorizontals());
        objects.extend(self.textboxverticals());
        objects.extend(self.textlinehorizontals());
        objects.extend(self.textlineverticals());
        PageLayout {
            page_number: self.page_number,
            bbox: self.bbox,
            objects,
        }
    }

    pub fn iter_layout_objects(&self) -> std::vec::IntoIter<LayoutObject> {
        let mut flat = Vec::new();
        fn push_children(target: &mut Vec<LayoutObject>, object: LayoutObject) {
            let children = object.children.clone();
            target.push(object);
            for child in children {
                push_children(target, child);
            }
        }
        for object in self.layout().objects {
            push_children(&mut flat, object);
        }
        flat.into_iter()
    }

    pub fn parse_objects(&self) -> BTreeMap<String, Vec<LayoutObject>> {
        let mut out = BTreeMap::new();
        let textboxh = self.textboxhorizontals();
        if !textboxh.is_empty() {
            out.insert("textboxhorizontal".to_string(), textboxh);
        }
        let textboxv = self.textboxverticals();
        if !textboxv.is_empty() {
            out.insert("textboxvertical".to_string(), textboxv);
        }
        let textlineh = self.textlinehorizontals();
        if !textlineh.is_empty() {
            out.insert("textlinehorizontal".to_string(), textlineh);
        }
        let textlinev = self.textlineverticals();
        if !textlinev.is_empty() {
            out.insert("textlinevertical".to_string(), textlinev);
        }
        out
    }

    pub fn process_object(&self, obj: &LayoutObject) -> LayoutObject {
        obj.clone()
    }

    pub fn structure_tree(&self) -> Option<&StructureElement> {
        self.structure_tree.as_ref()
    }

    pub fn derive_word_layout(&self) -> Vec<LayoutObject> {
        let options = self.default_text_options();
        extract_words(&self.chars, &options, false)
            .into_iter()
            .map(|word| {
                let bbox = BBox::new(word.x0, word.top, word.x1, word.bottom);
                layout_object_from_bbox(
                    "word",
                    self.page_number,
                    bbox,
                    Some(word.text),
                    Some(word.direction),
                    Some(word.upright),
                    Vec::new(),
                )
            })
            .collect()
    }
}

impl PdfDocument {
    pub fn structure_tree(&self) -> Option<&StructureElement> {
        self.structure_tree.as_ref()
    }

    pub fn textboxhorizontals(&self) -> Vec<LayoutObject> {
        self.pages.iter().flat_map(|page| page.textboxhorizontals()).collect()
    }

    pub fn textboxverticals(&self) -> Vec<LayoutObject> {
        self.pages.iter().flat_map(|page| page.textboxverticals()).collect()
    }

    pub fn textlinehorizontals(&self) -> Vec<LayoutObject> {
        self.pages.iter().flat_map(|page| page.textlinehorizontals()).collect()
    }

    pub fn textlineverticals(&self) -> Vec<LayoutObject> {
        self.pages.iter().flat_map(|page| page.textlineverticals()).collect()
    }
}
