use crate::error::{Error, Result};
use crate::geometry::bbox_from_points;
use crate::types::{
    Annotation, BBox, Char, Curve, Hyperlink, ImageObject, Line, Page, PathCommand, Point, RectObject,
};
use euclid::{Point2D, Transform2D};
use lopdf::content::Content;
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use pdf_extract::{MediaBox, OutputDev, Path, PathOp, Space, Transform};

#[derive(Debug, Clone, Copy)]
struct PageGeometry {
    raw_x0: f64,
    raw_y0: f64,
    raw_width: f64,
    raw_height: f64,
    rotation: i32,
    width: f64,
    height: f64,
    doctop_offset: f64,
}

impl PageGeometry {
    fn from_media_box(media_box: [f64; 4], rotation: i32, doctop_offset: f64) -> Self {
        let raw_x0 = media_box[0].min(media_box[2]);
        let raw_y0 = media_box[1].min(media_box[3]);
        let raw_x1 = media_box[0].max(media_box[2]);
        let raw_y1 = media_box[1].max(media_box[3]);
        let raw_width = raw_x1 - raw_x0;
        let raw_height = raw_y1 - raw_y0;
        let (width, height) = if rotation == 90 || rotation == 270 {
            (raw_height, raw_width)
        } else {
            (raw_width, raw_height)
        };
        Self {
            raw_x0,
            raw_y0,
            raw_width,
            raw_height,
            rotation,
            width,
            height,
            doctop_offset,
        }
    }

    fn page_bbox(self) -> BBox {
        BBox::new(0.0, 0.0, self.width, self.height)
    }

    fn map_raw_point(self, x: f64, y: f64) -> Point {
        let mut pt = (x - self.raw_x0, y - self.raw_y0);
        let turns = ((self.rotation % 360) + 360) % 360 / 90;
        for i in 0..turns {
            let (px, py) = pt;
            let comp = if i == turns % 2 { self.width } else { self.height };
            pt = (py, comp - px);
        }
        Point::new(pt.0, self.height - pt.1)
    }

    fn doctop(self, top: f64) -> f64 {
        self.doctop_offset + top
    }
}

struct CollectorOutput {
    geom: PageGeometry,
    page_number: usize,
    chars: Vec<Char>,
    lines: Vec<Line>,
    rects: Vec<RectObject>,
    curves: Vec<Curve>,
}

impl CollectorOutput {
    fn new(geom: PageGeometry, page_number: usize) -> Self {
        Self {
            geom,
            page_number,
            chars: Vec::new(),
            lines: Vec::new(),
            rects: Vec::new(),
            curves: Vec::new(),
        }
    }

    fn finish(self) -> (Vec<Char>, Vec<Line>, Vec<RectObject>, Vec<Curve>) {
        (self.chars, self.lines, self.rects, self.curves)
    }

    fn push_char(&mut self, trm: &Transform, width: f64, font_size: f64, text: &str) {
        let descent = font_size * 0.207;
        let ascent = font_size - descent;
        let advance = width.max(0.0) * font_size;

        let p0_raw = trm.transform_point(Point2D::<f64, Space>::new(0.0, -descent));
        let p1_raw = trm.transform_point(Point2D::<f64, Space>::new(advance, -descent));
        let p2_raw = trm.transform_point(Point2D::<f64, Space>::new(advance, ascent));
        let p3_raw = trm.transform_point(Point2D::<f64, Space>::new(0.0, ascent));

        let p0 = self.geom.map_raw_point(p0_raw.x, p0_raw.y);
        let p1 = self.geom.map_raw_point(p1_raw.x, p1_raw.y);
        let p2 = self.geom.map_raw_point(p2_raw.x, p2_raw.y);
        let p3 = self.geom.map_raw_point(p3_raw.x, p3_raw.y);

        let Some(bbox) = bbox_from_points(&[p0, p1, p2, p3]) else {
            return;
        };

        let baseline_dx = p1.x - p0.x;
        let baseline_dy = p1.y - p0.y;
        let upright = baseline_dx.abs() >= baseline_dy.abs();

        let vertical_dx = p3.x - p0.x;
        let vertical_dy = p3.y - p0.y;
        let size = (vertical_dx.powi(2) + vertical_dy.powi(2)).sqrt();

        self.chars.push(Char {
            text: text.to_string(),
            x0: bbox.x0,
            top: bbox.top,
            x1: bbox.x1,
            bottom: bbox.bottom,
            y0: self.geom.height - bbox.bottom,
            y1: self.geom.height - bbox.top,
            doctop: self.geom.doctop(bbox.top),
            width: bbox.width(),
            height: bbox.height(),
            size,
            adv: advance,
            upright,
            fontname: "unknown".to_string(),
            matrix: [trm.m11, trm.m12, trm.m21, trm.m22, trm.m31, trm.m32],
        });
    }

    fn push_path(&mut self, ctm: &Transform, path: &Path, stroke: bool, fill: bool) {
        if path.ops.is_empty() {
            return;
        }

        // Handle single rect
        if path.ops.len() == 1 {
            if let PathOp::Rect(x, y, w, h) = &path.ops[0] {
                let corners = [
                    self.map_transformed_point(ctm, *x, *y),
                    self.map_transformed_point(ctm, *x + *w, *y),
                    self.map_transformed_point(ctm, *x + *w, *y + *h),
                    self.map_transformed_point(ctm, *x, *y + *h),
                ];
                let Some(bbox) = bbox_from_points(&corners) else {
                    return;
                };
                self.rects.push(RectObject {
                    x0: bbox.x0,
                    top: bbox.top,
                    x1: bbox.x1,
                    bottom: bbox.bottom,
                    y0: self.geom.height - bbox.bottom,
                    y1: self.geom.height - bbox.top,
                    doctop: self.geom.doctop(bbox.top),
                    width: bbox.width(),
                    height: bbox.height(),
                    pts: corners.to_vec(),
                    path: vec![PathCommand::Rect {
                        x: bbox.x0,
                        y: bbox.top,
                        width: bbox.width(),
                        height: bbox.height(),
                    }],
                    stroke,
                    fill,
                    linewidth: 0.0,
                });
                return;
            }
        }

        // Handle simple straight line (MoveTo+LineTo or MoveTo+LineTo+Close)
        if let Some((start, end)) = straight_line_from_path(path) {
            let p0 = self.map_transformed_point(ctm, start.0, start.1);
            let p1 = self.map_transformed_point(ctm, end.0, end.1);
            let Some(bbox) = bbox_from_points(&[p0, p1]) else {
                return;
            };
            self.lines.push(Line {
                x0: bbox.x0,
                top: bbox.top,
                x1: bbox.x1,
                bottom: bbox.bottom,
                y0: self.geom.height - bbox.bottom,
                y1: self.geom.height - bbox.top,
                doctop: self.geom.doctop(bbox.top),
                width: bbox.width(),
                height: bbox.height(),
                pts: vec![p0, p1],
                stroke,
                fill,
                linewidth: 0.0,
            });
            return;
        }

        // Complex path: split into subpaths at MoveTo boundaries.
        // Subpaths with exactly one LineTo (2-point paths) become lines.
        // The overall path is always emitted as a curve.
        //
        // First pass: collect subpaths to find 2-point line segments
        {
            let mut sub_start: Option<(f64, f64)> = None;
            let mut sub_segments: Vec<(f64, f64, f64, f64)> = Vec::new();
            let mut sub_has_curve = false;

            let flush_subpath =
                |_sub_start: &Option<(f64, f64)>,
                 sub_segments: &[(f64, f64, f64, f64)],
                 sub_has_curve: bool,
                 collector: &mut Self| {
                    // Only emit a line if this subpath is a single straight segment
                    if !sub_has_curve && sub_segments.len() == 1 {
                        let (x0, y0, x1, y1) = sub_segments[0];
                        collector.push_line_segment(ctm, x0, y0, x1, y1, stroke, fill);
                    }
                    // Also check: MoveTo + Close with no LineTo is degenerate, skip
                    // MoveTo + multiple LineTo = polygon, keep as part of curve only
                };

            for op in &path.ops {
                match op {
                    PathOp::MoveTo(x, y) => {
                        flush_subpath(&sub_start, &sub_segments, sub_has_curve, self);
                        sub_start = Some((*x, *y));
                        sub_segments.clear();
                        sub_has_curve = false;
                    }
                    PathOp::LineTo(x, y) => {
                        let prev = sub_segments
                            .last()
                            .map(|(_, _, ex, ey)| (*ex, *ey))
                            .or(sub_start)
                            .unwrap_or((0.0, 0.0));
                        sub_segments.push((prev.0, prev.1, *x, *y));
                    }
                    PathOp::CurveTo(..) => {
                        sub_has_curve = true;
                    }
                    PathOp::Close => {
                        // Close back to subpath start
                        if let Some((sx, sy)) = sub_start {
                            let prev = sub_segments
                                .last()
                                .map(|(_, _, ex, ey)| (*ex, *ey))
                                .or(sub_start)
                                .unwrap_or((0.0, 0.0));
                            if (prev.0 - sx).abs() > 1e-6 || (prev.1 - sy).abs() > 1e-6 {
                                sub_segments.push((prev.0, prev.1, sx, sy));
                            }
                        }
                        flush_subpath(&sub_start, &sub_segments, sub_has_curve, self);
                        sub_segments.clear();
                        sub_has_curve = false;
                        // sub_start stays the same for potential next ops
                    }
                    PathOp::Rect(..) => {}
                }
            }
            flush_subpath(&sub_start, &sub_segments, sub_has_curve, self);
        }

        // Second pass: build the curve object
        let mut pts: Vec<Point> = Vec::new();
        let mut commands = Vec::new();
        let mut current: Option<(f64, f64)> = None;

        for op in &path.ops {
            match op {
                PathOp::MoveTo(x, y) => {
                    let p = self.map_transformed_point(ctm, *x, *y);
                    pts.push(p);
                    commands.push(PathCommand::MoveTo(p));
                    current = Some((*x, *y));
                }
                PathOp::LineTo(x, y) => {
                    let p = self.map_transformed_point(ctm, *x, *y);
                    pts.push(p);
                    commands.push(PathCommand::LineTo(p));
                    current = Some((*x, *y));
                }
                PathOp::CurveTo(x1, y1, x2, y2, x3, y3) => {
                    let c1 = self.map_transformed_point(ctm, *x1, *y1);
                    let c2 = self.map_transformed_point(ctm, *x2, *y2);
                    let p = self.map_transformed_point(ctm, *x3, *y3);
                    if pts.is_empty() {
                        if let Some((cx, cy)) = current {
                            pts.push(self.map_transformed_point(ctm, cx, cy));
                        }
                    }
                    pts.push(p);
                    commands.push(PathCommand::CurveTo { c1, c2, p });
                    current = Some((*x3, *y3));
                }
                PathOp::Rect(x, y, w, h) => {
                    commands.push(PathCommand::Rect { x: *x, y: *y, width: *w, height: *h });
                }
                PathOp::Close => commands.push(PathCommand::Close),
            }
        }

        let Some(bbox) = bbox_from_points(&pts) else {
            return;
        };

        self.curves.push(Curve {
            x0: bbox.x0,
            top: bbox.top,
            x1: bbox.x1,
            bottom: bbox.bottom,
            y0: self.geom.height - bbox.bottom,
            y1: self.geom.height - bbox.top,
            doctop: self.geom.doctop(bbox.top),
            width: bbox.width(),
            height: bbox.height(),
            pts,
            path: commands,
            stroke,
            fill,
            linewidth: 0.0,
        });
    }

    fn push_line_segment(&mut self, ctm: &Transform, x0: f64, y0: f64, x1: f64, y1: f64, stroke: bool, fill: bool) {
        let p0 = self.map_transformed_point(ctm, x0, y0);
        let p1 = self.map_transformed_point(ctm, x1, y1);
        let Some(bbox) = bbox_from_points(&[p0, p1]) else {
            return;
        };
        self.lines.push(Line {
            x0: bbox.x0,
            top: bbox.top,
            x1: bbox.x1,
            bottom: bbox.bottom,
            y0: self.geom.height - bbox.bottom,
            y1: self.geom.height - bbox.top,
            doctop: self.geom.doctop(bbox.top),
            width: bbox.width(),
            height: bbox.height(),
            pts: vec![p0, p1],
            stroke,
            fill,
            linewidth: 0.0,
        });
    }

    fn map_transformed_point(&self, ctm: &Transform, x: f64, y: f64) -> Point {
        let raw = ctm.transform_point(Point2D::<f64, Space>::new(x, y));
        self.geom.map_raw_point(raw.x, raw.y)
    }
}

impl OutputDev for CollectorOutput {
    fn begin_page(&mut self, _page_num: u32, _media_box: &MediaBox, _art_box: Option<(f64, f64, f64, f64)>) -> std::result::Result<(), pdf_extract::OutputError> {
        Ok(())
    }

    fn end_page(&mut self) -> std::result::Result<(), pdf_extract::OutputError> {
        Ok(())
    }

    fn output_character(
        &mut self,
        trm: &Transform,
        width: f64,
        _spacing: f64,
        font_size: f64,
        text: &str,
    ) -> std::result::Result<(), pdf_extract::OutputError> {
        self.push_char(trm, width, font_size, text);
        Ok(())
    }

    fn begin_word(&mut self) -> std::result::Result<(), pdf_extract::OutputError> {
        Ok(())
    }

    fn end_word(&mut self) -> std::result::Result<(), pdf_extract::OutputError> {
        Ok(())
    }

    fn end_line(&mut self) -> std::result::Result<(), pdf_extract::OutputError> {
        Ok(())
    }

    fn stroke(
        &mut self,
        ctm: &Transform,
        _colorspace: &pdf_extract::ColorSpace,
        _color: &[f64],
        path: &Path,
    ) -> std::result::Result<(), pdf_extract::OutputError> {
        self.push_path(ctm, path, true, false);
        Ok(())
    }

    fn fill(
        &mut self,
        ctm: &Transform,
        _colorspace: &pdf_extract::ColorSpace,
        _color: &[f64],
        path: &Path,
    ) -> std::result::Result<(), pdf_extract::OutputError> {
        self.push_path(ctm, path, false, true);
        Ok(())
    }
}

fn straight_line_from_path(path: &Path) -> Option<((f64, f64), (f64, f64))> {
    if path.ops.len() == 2 {
        match (&path.ops[0], &path.ops[1]) {
            (PathOp::MoveTo(x0, y0), PathOp::LineTo(x1, y1)) => Some(((*x0, *y0), (*x1, *y1))),
            _ => None,
        }
    } else if path.ops.len() == 3 {
        match (&path.ops[0], &path.ops[1], &path.ops[2]) {
            (PathOp::MoveTo(x0, y0), PathOp::LineTo(x1, y1), PathOp::Close) => {
                Some(((*x0, *y0), (*x1, *y1)))
            }
            _ => None,
        }
    } else {
        None
    }
}

pub fn open_pdf<P: AsRef<std::path::Path>>(path: P) -> Result<crate::types::PdfDocument> {
    let pathbuf = path.as_ref().to_path_buf();
    let doc = Document::load(&pathbuf)?;
    let pages = doc.get_pages();

    let mut parsed_pages = Vec::new();
    let mut doctop_offset = 0.0;

    let mut ordered: Vec<(u32, ObjectId)> = pages.into_iter().collect();
    ordered.sort_by_key(|(page_number, _)| *page_number);

    for (page_number, page_id) in ordered {
        let page = parse_page(&doc, page_number as usize, page_id, doctop_offset)?;
        doctop_offset += page.height;
        parsed_pages.push(page);
    }

    Ok(crate::types::PdfDocument {
        path: pathbuf,
        pages: parsed_pages,
    })
}

fn parse_page(doc: &Document, page_number: usize, page_id: ObjectId, doctop_offset: f64) -> Result<Page> {
    let rotation = get_inherited_object(doc, page_id, b"Rotate")?
        .and_then(|obj| obj_to_i64(&obj))
        .unwrap_or(0) as i32
        % 360;

    let media_box_obj = get_inherited_object(doc, page_id, b"MediaBox")?
        .ok_or_else(|| Error::Message(format!("page {page_number} missing MediaBox")))?;
    let media_box = obj_to_box(&media_box_obj)?;
    let geom = PageGeometry::from_media_box(media_box, rotation, doctop_offset);

    let mut collector = CollectorOutput::new(geom, page_number);
    // pdf-extract may panic on malformed content streams; catch and convert to error
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pdf_extract::output_doc_page(doc, &mut collector, page_number as u32)
    }));
    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => {
            return Err(Error::Message(format!(
                "pdf-extract panicked while parsing page {page_number}"
            )));
        }
    }
    let (chars, lines, rects, curves) = collector.finish();

    let page_dict = page_dict(doc, page_id)?;
    let resources = get_inherited_object(doc, page_id, b"Resources")?
        .map(|obj| object_to_dict(doc, &obj))
        .transpose()?
        .unwrap_or_else(Dictionary::new);

    let images = collect_images(doc, &resources, page_id, geom, page_number)?;
    let (annots, hyperlinks) = collect_annotations(doc, &page_dict, geom)?;

    Ok(Page {
        page_number,
        rotation,
        width: geom.width,
        height: geom.height,
        bbox: geom.page_bbox(),
        doctop_offset,
        chars,
        lines,
        rects,
        curves,
        images,
        annots,
        hyperlinks,
    })
}

fn collect_images(
    doc: &Document,
    resources: &Dictionary,
    page_id: ObjectId,
    geom: PageGeometry,
    _page_number: usize,
) -> Result<Vec<ImageObject>> {
    let content = doc.get_page_content(page_id)?;
    let mut walker = ImageWalker {
        doc,
        geom,
        images: Vec::new(),
    };
    walker.walk_stream(content, resources, Transform2D::<f64, Space, Space>::identity())?;
    Ok(walker.images)
}

struct ImageWalker<'a> {
    doc: &'a Document,
    geom: PageGeometry,
    images: Vec<ImageObject>,
}

impl<'a> ImageWalker<'a> {
    fn walk_stream(&mut self, content: Vec<u8>, resources: &Dictionary, initial_ctm: Transform) -> Result<()> {
        let content = Content::decode(&content)?;
        let mut ctm = initial_ctm;
        let mut stack: Vec<Transform> = Vec::new();

        for op in content.operations {
            match op.operator.as_str() {
                "q" => stack.push(ctm),
                "Q" => {
                    ctm = stack.pop().unwrap_or_else(Transform2D::<f64, Space, Space>::identity);
                }
                "cm" => {
                    if op.operands.len() == 6 {
                        let m = transform_from_operands(&op.operands)?;
                        ctm = ctm.pre_transform(&m);
                    }
                }
                "Do" => {
                    if let Some(name) = op.operands.get(0).and_then(obj_to_name_string) {
                        self.handle_xobject(resources, &name, ctm)?;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_xobject(&mut self, resources: &Dictionary, name: &str, ctm: Transform) -> Result<()> {
        let Some(xobjects_obj) = dict_get(resources, b"XObject") else {
            return Ok(());
        };
        let xobjects = object_to_dict(self.doc, xobjects_obj)?;
        let Some(target) = dict_get(&xobjects, name.as_bytes()) else {
            return Ok(());
        };

        let object = deref_object(self.doc, target)?;
        let stream = match object {
            Object::Stream(stream) => stream,
            _ => return Ok(()),
        };

        let subtype = dict_get(&stream.dict, b"Subtype")
            .and_then(obj_to_name_string)
            .unwrap_or_default();

        match subtype.as_str() {
            "Image" => self.push_image(name, &stream, ctm),
            "Form" => {
                let form_resources = dict_get(&stream.dict, b"Resources")
                    .map(|obj| object_to_dict(self.doc, obj))
                    .transpose()?
                    .unwrap_or_else(|| resources.clone());

                let form_matrix = dict_get(&stream.dict, b"Matrix")
                    .map(transform_from_obj)
                    .transpose()?;

                let next_ctm = if let Some(matrix) = form_matrix {
                    ctm.pre_transform(&matrix)
                } else {
                    ctm
                };

                let bytes = stream
                    .decompressed_content()
                    .unwrap_or_else(|_| stream.content.clone());

                self.walk_stream(bytes, &form_resources, next_ctm)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn push_image(&mut self, name: &str, stream: &Stream, ctm: Transform) {
        let corners = [
            self.map_image_point(&ctm, 0.0, 0.0),
            self.map_image_point(&ctm, 1.0, 0.0),
            self.map_image_point(&ctm, 1.0, 1.0),
            self.map_image_point(&ctm, 0.0, 1.0),
        ];

        let Some(bbox) = bbox_from_points(&corners) else {
            return;
        };

        let width = dict_get(&stream.dict, b"Width")
            .and_then(obj_to_i64)
            .unwrap_or(0) as u32;
        let height = dict_get(&stream.dict, b"Height")
            .and_then(obj_to_i64)
            .unwrap_or(0) as u32;
        let bits = dict_get(&stream.dict, b"BitsPerComponent").and_then(obj_to_i64);

        let colorspace = dict_get(&stream.dict, b"ColorSpace").map(color_space_name);

        self.images.push(ImageObject {
            x0: bbox.x0,
            top: bbox.top,
            x1: bbox.x1,
            bottom: bbox.bottom,
            y0: self.geom.height - bbox.bottom,
            y1: self.geom.height - bbox.top,
            doctop: self.geom.doctop(bbox.top),
            width: bbox.width(),
            height: bbox.height(),
            name: name.to_string(),
            srcsize: (width, height),
            bits,
            colorspace,
        });
    }

    fn map_image_point(&self, ctm: &Transform, x: f64, y: f64) -> Point {
        let raw = ctm.transform_point(Point2D::<f64, Space>::new(x, y));
        self.geom.map_raw_point(raw.x, raw.y)
    }
}

fn collect_annotations(doc: &Document, page_dict: &Dictionary, geom: PageGeometry) -> Result<(Vec<Annotation>, Vec<Hyperlink>)> {
    let mut annots = Vec::new();
    let mut hyperlinks = Vec::new();

    let Some(annots_obj) = dict_get(page_dict, b"Annots") else {
        return Ok((annots, hyperlinks));
    };
    let annots_array = match deref_object(doc, annots_obj)? {
        Object::Array(items) => items,
        _ => return Ok((annots, hyperlinks)),
    };

    for item in annots_array {
        let object = deref_object(doc, &item)?;
        let dict = match object {
            Object::Dictionary(dict) => dict,
            _ => continue,
        };

        let rect = match dict_get(&dict, b"Rect") {
            Some(obj) => obj_to_box(obj).ok(),
            None => None,
        };
        let Some(raw_rect) = rect else {
            continue;
        };

        let p0 = geom.map_raw_point(raw_rect[0], raw_rect[1]);
        let p1 = geom.map_raw_point(raw_rect[2], raw_rect[3]);
        let Some(bbox) = bbox_from_points(&[p0, p1]) else {
            continue;
        };

        let subtype = dict_get(&dict, b"Subtype")
            .and_then(obj_to_name_string)
            .unwrap_or_else(|| "Annot".to_string());

        let action_uri = dict_get(&dict, b"A")
            .and_then(|obj| object_to_dict(doc, obj).ok())
            .and_then(|action| dict_get(&action, b"URI").and_then(decode_pdf_string));

        let title = dict_get(&dict, b"T").and_then(decode_pdf_string);
        let contents = dict_get(&dict, b"Contents").and_then(decode_pdf_string);

        let annot = Annotation {
            x0: bbox.x0,
            top: bbox.top,
            x1: bbox.x1,
            bottom: bbox.bottom,
            y0: geom.height - bbox.bottom,
            y1: geom.height - bbox.top,
            doctop: geom.doctop(bbox.top),
            width: bbox.width(),
            height: bbox.height(),
            subtype,
            uri: action_uri.clone(),
            title,
            contents,
        };

        if let Some(uri) = action_uri {
            hyperlinks.push(Hyperlink {
                x0: annot.x0,
                top: annot.top,
                x1: annot.x1,
                bottom: annot.bottom,
                y0: annot.y0,
                y1: annot.y1,
                doctop: annot.doctop,
                width: annot.width,
                height: annot.height,
                uri,
            });
        }

        annots.push(annot);
    }

    Ok((annots, hyperlinks))
}

fn page_dict(doc: &Document, page_id: ObjectId) -> Result<Dictionary> {
    match doc.get_object(page_id)? {
        Object::Dictionary(dict) => Ok(dict.clone()),
        Object::Stream(stream) => Ok(stream.dict.clone()),
        other => Err(Error::Type(format!("page object is not a dictionary: {:?}", other))),
    }
}

fn get_inherited_object(doc: &Document, mut current_id: ObjectId, key: &[u8]) -> Result<Option<Object>> {
    loop {
        let dict = page_dict(doc, current_id)?;
        if let Some(obj) = dict_get(&dict, key) {
            return Ok(Some(deref_object(doc, obj)?));
        }
        let parent = dict_get(&dict, b"Parent").and_then(obj_to_reference);
        if let Some(parent_id) = parent {
            current_id = parent_id;
        } else {
            return Ok(None);
        }
    }
}

fn transform_from_operands(operands: &[Object]) -> Result<Transform> {
    if operands.len() != 6 {
        return Err(Error::Message("transform expects 6 operands".to_string()));
    }
    Ok(Transform2D::<f64, Space, Space>::row_major(
        obj_to_f64(&operands[0]).ok_or_else(|| Error::Type("transform operand a".to_string()))?,
        obj_to_f64(&operands[1]).ok_or_else(|| Error::Type("transform operand b".to_string()))?,
        obj_to_f64(&operands[2]).ok_or_else(|| Error::Type("transform operand c".to_string()))?,
        obj_to_f64(&operands[3]).ok_or_else(|| Error::Type("transform operand d".to_string()))?,
        obj_to_f64(&operands[4]).ok_or_else(|| Error::Type("transform operand e".to_string()))?,
        obj_to_f64(&operands[5]).ok_or_else(|| Error::Type("transform operand f".to_string()))?,
    ))
}

fn transform_from_obj(obj: &Object) -> Result<Transform> {
    match obj {
        Object::Array(items) => transform_from_operands(items),
        other => Err(Error::Type(format!("expected transform array, got {:?}", other))),
    }
}

fn obj_to_box(obj: &Object) -> Result<[f64; 4]> {
    match obj {
        Object::Array(items) if items.len() == 4 => Ok([
            obj_to_f64(&items[0]).ok_or_else(|| Error::Type("bbox item 0".to_string()))?,
            obj_to_f64(&items[1]).ok_or_else(|| Error::Type("bbox item 1".to_string()))?,
            obj_to_f64(&items[2]).ok_or_else(|| Error::Type("bbox item 2".to_string()))?,
            obj_to_f64(&items[3]).ok_or_else(|| Error::Type("bbox item 3".to_string()))?,
        ]),
        other => Err(Error::Type(format!("expected bbox array, got {:?}", other))),
    }
}

fn obj_to_f64(obj: &Object) -> Option<f64> {
    match obj {
        Object::Integer(value) => Some(*value as f64),
        Object::Real(value) => Some(*value as f64),
        _ => None,
    }
}

fn obj_to_i64(obj: &Object) -> Option<i64> {
    match obj {
        Object::Integer(value) => Some(*value),
        Object::Real(value) => Some(*value as i64),
        _ => None,
    }
}

fn obj_to_reference(obj: &Object) -> Option<ObjectId> {
    match obj {
        Object::Reference(id) => Some(*id),
        _ => None,
    }
}

fn obj_to_name_string(obj: &Object) -> Option<String> {
    match obj {
        Object::Name(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        _ => None,
    }
}

fn color_space_name(obj: &Object) -> String {
    match obj {
        Object::Name(bytes) => String::from_utf8_lossy(bytes).into_owned(),
        Object::Array(items) => items
            .first()
            .and_then(obj_to_name_string)
            .unwrap_or_else(|| "unknown".to_string()),
        _ => "unknown".to_string(),
    }
}

fn decode_pdf_string(obj: &Object) -> Option<String> {
    match obj {
        Object::String(bytes, _) => decode_bytes(bytes),
        Object::Name(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        _ => None,
    }
}

fn decode_bytes(bytes: &[u8]) -> Option<String> {
    if bytes.starts_with(&[0xFE, 0xFF]) && bytes.len() % 2 == 0 {
        let mut units = Vec::new();
        let mut idx = 2usize;
        while idx + 1 < bytes.len() {
            units.push(u16::from_be_bytes([bytes[idx], bytes[idx + 1]]));
            idx += 2;
        }
        String::from_utf16(&units).ok()
    } else {
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}

fn deref_object(doc: &Document, obj: &Object) -> Result<Object> {
    match obj {
        Object::Reference(id) => Ok(doc.get_object(*id)?.clone()),
        _ => Ok(obj.clone()),
    }
}

fn object_to_dict(doc: &Document, obj: &Object) -> Result<Dictionary> {
    match deref_object(doc, obj)? {
        Object::Dictionary(dict) => Ok(dict),
        Object::Stream(stream) => Ok(stream.dict),
        other => Err(Error::Type(format!("expected dictionary, got {:?}", other))),
    }
}

fn dict_get<'a>(dict: &'a Dictionary, key: &[u8]) -> Option<&'a Object> {
    dict.get(key).ok()
}
