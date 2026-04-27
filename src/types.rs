use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub type JsonMap = BTreeMap<String, Value>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct BBox {
    pub x0: f64,
    pub top: f64,
    pub x1: f64,
    pub bottom: f64,
}

impl BBox {
    pub const fn new(x0: f64, top: f64, x1: f64, bottom: f64) -> Self {
        Self { x0, top, x1, bottom }
    }

    pub fn width(self) -> f64 {
        self.x1 - self.x0
    }

    pub fn height(self) -> f64 {
        self.bottom - self.top
    }

    pub fn area(self) -> f64 {
        self.width() * self.height()
    }

    pub fn is_valid(self) -> bool {
        self.x0 <= self.x1 && self.top <= self.bottom && self.is_finite()
    }

    pub fn is_finite(self) -> bool {
        self.x0.is_finite() && self.top.is_finite() && self.x1.is_finite() && self.bottom.is_finite()
    }

    pub fn is_empty(self) -> bool {
        self.width() == 0.0 || self.height() == 0.0
    }

    pub fn normalized(self) -> Self {
        Self::new(
            self.x0.min(self.x1),
            self.top.min(self.bottom),
            self.x0.max(self.x1),
            self.top.max(self.bottom),
        )
    }

    pub fn overlaps(self, other: Self) -> bool {
        self.overlap(other).is_some()
    }

    pub fn intersects(self, other: Self) -> bool {
        self.overlaps(other)
    }

    pub fn contains_point(self, point: Point) -> bool {
        let bbox = self.normalized();
        point.x >= bbox.x0 && point.x <= bbox.x1 && point.y >= bbox.top && point.y <= bbox.bottom
    }

    pub fn contains_bbox(self, other: Self) -> bool {
        let bbox = self.normalized();
        let other = other.normalized();
        bbox.x0 <= other.x0
            && bbox.top <= other.top
            && bbox.x1 >= other.x1
            && bbox.bottom >= other.bottom
    }

    pub fn overlap(self, other: Self) -> Option<Self> {
        let bbox = self.normalized();
        let other = other.normalized();
        if !bbox.is_finite() || !other.is_finite() {
            return None;
        }

        let x0 = bbox.x0.max(other.x0);
        let top = bbox.top.max(other.top);
        let x1 = bbox.x1.min(other.x1);
        let bottom = bbox.bottom.min(other.bottom);
        if x1 >= x0 && bottom >= top && ((x1 - x0) + (bottom - top) > 0.0) {
            Some(Self::new(x0, top, x1, bottom))
        } else {
            None
        }
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        self.overlap(other)
    }

    pub fn intersection_area(self, other: Self) -> f64 {
        self.overlap(other)
            .map(|bbox| bbox.width().max(0.0) * bbox.height().max(0.0))
            .unwrap_or(0.0)
    }

    pub fn overlap_ratio(self, other: Self) -> f64 {
        let area = self.normalized().area();
        if area <= 0.0 {
            0.0
        } else {
            self.intersection_area(other) / area
        }
    }

    pub fn intersection_over_union(self, other: Self) -> f64 {
        let a = self.normalized().area().max(0.0);
        let b = other.normalized().area().max(0.0);
        let intersection = self.intersection_area(other);
        let union = a + b - intersection;
        if union <= 0.0 {
            0.0
        } else {
            intersection / union
        }
    }

    pub fn union(self, other: Self) -> Self {
        let bbox = self.normalized();
        let other = other.normalized();
        Self::new(
            bbox.x0.min(other.x0),
            bbox.top.min(other.top),
            bbox.x1.max(other.x1),
            bbox.bottom.max(other.bottom),
        )
    }

    pub fn clamp(self, bounds: Self) -> Option<Self> {
        self.overlap(bounds)
    }

    pub fn expand(self, dx: f64, dy: f64) -> Self {
        Self::new(self.x0 - dx, self.top - dy, self.x1 + dx, self.bottom + dy)
    }

    pub fn pad(self, amount: f64) -> Self {
        self.expand(amount, amount)
    }

    pub fn round(self, precision: usize) -> Self {
        let factor = 10f64.powi(precision as i32);
        Self::new(
            (self.x0 * factor).round() / factor,
            (self.top * factor).round() / factor,
            (self.x1 * factor).round() / factor,
            (self.bottom * factor).round() / factor,
        )
    }

    pub fn translate(self, dx: f64, dy: f64) -> Self {
        Self::new(self.x0 + dx, self.top + dy, self.x1 + dx, self.bottom + dy)
    }

    pub fn as_tuple(self) -> (f64, f64, f64, f64) {
        (self.x0, self.top, self.x1, self.bottom)
    }

    pub fn center(self) -> Point {
        Point::new((self.x0 + self.x1) / 2.0, (self.top + self.bottom) / 2.0)
    }
}

impl Default for BBox {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Ttb,
    Btt,
    Ltr,
    Rtl,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ttb => "ttb",
            Self::Btt => "btt",
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }

    pub fn is_horizontal(self) -> bool {
        matches!(self, Self::Ltr | Self::Rtl)
    }

    pub fn is_vertical(self) -> bool {
        matches!(self, Self::Ttb | Self::Btt)
    }
}

impl std::str::FromStr for Direction {
    type Err = crate::Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "ttb" => Ok(Self::Ttb),
            "btt" => Ok(Self::Btt),
            "ltr" => Ok(Self::Ltr),
            "rtl" => Ok(Self::Rtl),
            other => Err(crate::Error::Message(format!("unknown direction: {other}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Horizontal,
    Vertical,
}

impl Orientation {
    pub fn as_char(self) -> &'static str {
        match self {
            Self::Horizontal => "h",
            Self::Vertical => "v",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Char {
    pub object_type: String,
    pub page_number: usize,
    pub text: String,
    pub x0: f64,
    pub top: f64,
    pub x1: f64,
    pub bottom: f64,
    pub y0: f64,
    pub y1: f64,
    pub doctop: f64,
    pub width: f64,
    pub height: f64,
    pub size: f64,
    pub adv: f64,
    pub upright: bool,
    pub fontname: String,
    pub matrix: [f64; 6],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcid: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ncs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroking_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_stroking_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Word {
    pub text: String,
    pub x0: f64,
    pub top: f64,
    pub x1: f64,
    pub bottom: f64,
    pub doctop: f64,
    pub width: f64,
    pub height: f64,
    pub upright: bool,
    pub direction: Direction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chars: Option<Vec<Char>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Line {
    pub object_type: String,
    pub page_number: usize,
    pub x0: f64,
    pub top: f64,
    pub x1: f64,
    pub bottom: f64,
    pub y0: f64,
    pub y1: f64,
    pub doctop: f64,
    pub width: f64,
    pub height: f64,
    pub pts: Vec<Point>,
    pub stroke: bool,
    pub fill: bool,
    pub linewidth: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PathCommand {
    MoveTo(Point),
    LineTo(Point),
    CurveTo { c1: Point, c2: Point, p: Point },
    Rect { x: f64, y: f64, width: f64, height: f64 },
    Close,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RectObject {
    pub object_type: String,
    pub page_number: usize,
    pub x0: f64,
    pub top: f64,
    pub x1: f64,
    pub bottom: f64,
    pub y0: f64,
    pub y1: f64,
    pub doctop: f64,
    pub width: f64,
    pub height: f64,
    pub pts: Vec<Point>,
    pub path: Vec<PathCommand>,
    pub stroke: bool,
    pub fill: bool,
    pub linewidth: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Curve {
    pub object_type: String,
    pub page_number: usize,
    pub x0: f64,
    pub top: f64,
    pub x1: f64,
    pub bottom: f64,
    pub y0: f64,
    pub y1: f64,
    pub doctop: f64,
    pub width: f64,
    pub height: f64,
    pub pts: Vec<Point>,
    pub path: Vec<PathCommand>,
    pub stroke: bool,
    pub fill: bool,
    pub linewidth: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageObject {
    pub object_type: String,
    pub page_number: usize,
    pub x0: f64,
    pub top: f64,
    pub x1: f64,
    pub bottom: f64,
    pub y0: f64,
    pub y1: f64,
    pub doctop: f64,
    pub width: f64,
    pub height: f64,
    pub name: String,
    pub srcsize: (u32, u32),
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bits: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colorspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imagemask: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcid: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Annotation {
    pub object_type: String,
    pub page_number: usize,
    pub x0: f64,
    pub top: f64,
    pub x1: f64,
    pub bottom: f64,
    pub y0: f64,
    pub y1: f64,
    pub doctop: f64,
    pub width: f64,
    pub height: f64,
    pub subtype: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hyperlink {
    pub object_type: String,
    pub page_number: usize,
    pub x0: f64,
    pub top: f64,
    pub x1: f64,
    pub bottom: f64,
    pub y0: f64,
    pub y1: f64,
    pub doctop: f64,
    pub width: f64,
    pub height: f64,
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    pub x0: f64,
    pub top: f64,
    pub x1: f64,
    pub bottom: f64,
    pub width: f64,
    pub height: f64,
    pub orientation: Orientation,
    pub object_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ObjectCounts {
    pub chars: usize,
    pub lines: usize,
    pub rects: usize,
    pub curves: usize,
    pub images: usize,
    pub annots: usize,
    pub hyperlinks: usize,
}

impl ObjectCounts {
    pub fn total(&self) -> usize {
        self.chars + self.lines + self.rects + self.curves + self.images + self.annots + self.hyperlinks
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

impl std::ops::Add for ObjectCounts {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            chars: self.chars + rhs.chars,
            lines: self.lines + rhs.lines,
            rects: self.rects + rhs.rects,
            curves: self.curves + rhs.curves,
            images: self.images + rhs.images,
            annots: self.annots + rhs.annots,
            hyperlinks: self.hyperlinks + rhs.hyperlinks,
        }
    }
}

impl std::ops::AddAssign for ObjectCounts {
    fn add_assign(&mut self, rhs: Self) {
        self.chars += rhs.chars;
        self.lines += rhs.lines;
        self.rects += rhs.rects;
        self.curves += rhs.curves;
        self.images += rhs.images;
        self.annots += rhs.annots;
        self.hyperlinks += rhs.hyperlinks;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchMatch {
    pub text: String,
    pub x0: f64,
    pub top: f64,
    pub x1: f64,
    pub bottom: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<Option<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chars: Option<Vec<Char>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextLine {
    pub text: String,
    pub x0: f64,
    pub top: f64,
    pub x1: f64,
    pub bottom: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chars: Option<Vec<Char>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutObject {
    pub object_type: String,
    pub page_number: usize,
    pub x0: f64,
    pub top: f64,
    pub x1: f64,
    pub bottom: f64,
    pub width: f64,
    pub height: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upright: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<LayoutObject>,
}

impl LayoutObject {
    pub fn bbox(&self) -> BBox {
        BBox::new(self.x0, self.top, self.x1, self.bottom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageLayout {
    pub page_number: usize,
    pub bbox: BBox,
    pub objects: Vec<LayoutObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StructureElement {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcid: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<StructureElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Page {
    pub page_number: usize,
    pub rotation: i32,
    pub width: f64,
    pub height: f64,
    pub bbox: BBox,
    pub mediabox: BBox,
    pub cropbox: BBox,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trimbox: Option<BBox>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bleedbox: Option<BBox>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artbox: Option<BBox>,
    pub doctop_offset: f64,
    pub is_original: bool,
    pub chars: Vec<Char>,
    pub lines: Vec<Line>,
    pub rects: Vec<RectObject>,
    pub curves: Vec<Curve>,
    pub images: Vec<ImageObject>,
    pub annots: Vec<Annotation>,
    pub hyperlinks: Vec<Hyperlink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structure_tree: Option<StructureElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PdfDocument {
    pub path: PathBuf,
    pub pages: Vec<Page>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: JsonMap,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structure_tree: Option<StructureElement>,
}

pub enum PageObjectRef<'a> {
    Char(&'a Char),
    Line(&'a Line),
    Rect(&'a RectObject),
    Curve(&'a Curve),
    Image(&'a ImageObject),
    Annot(&'a Annotation),
    Hyperlink(&'a Hyperlink),
}

impl<'a> PageObjectRef<'a> {
    pub fn object_type(&self) -> &'static str {
        match self {
            Self::Char(_) => "char",
            Self::Line(_) => "line",
            Self::Rect(_) => "rect",
            Self::Curve(_) => "curve",
            Self::Image(_) => "image",
            Self::Annot(_) => "annot",
            Self::Hyperlink(_) => "hyperlink",
        }
    }

    pub fn page_number(&self) -> usize {
        match self {
            Self::Char(item) => item.page_number,
            Self::Line(item) => item.page_number,
            Self::Rect(item) => item.page_number,
            Self::Curve(item) => item.page_number,
            Self::Image(item) => item.page_number,
            Self::Annot(item) => item.page_number,
            Self::Hyperlink(item) => item.page_number,
        }
    }

    pub fn bbox(&self) -> BBox {
        match self {
            Self::Char(item) => item.bbox(),
            Self::Line(item) => item.bbox(),
            Self::Rect(item) => item.bbox(),
            Self::Curve(item) => item.bbox(),
            Self::Image(item) => item.bbox(),
            Self::Annot(item) => item.bbox(),
            Self::Hyperlink(item) => item.bbox(),
        }
    }
}

pub trait Bounded: Clone {
    fn bbox(&self) -> BBox;
    fn with_bbox(&self, bbox: BBox, page_height: f64) -> Self;
}

macro_rules! impl_bounded {
    ($ty:ty) => {
        impl Bounded for $ty {
            fn bbox(&self) -> BBox {
                BBox::new(self.x0, self.top, self.x1, self.bottom)
            }

            fn with_bbox(&self, bbox: BBox, page_height: f64) -> Self {
                let mut copy = self.clone();
                copy.x0 = bbox.x0;
                copy.top = bbox.top;
                copy.x1 = bbox.x1;
                copy.bottom = bbox.bottom;
                copy.width = bbox.width();
                copy.height = bbox.height();
                copy.y0 = page_height - bbox.bottom;
                copy.y1 = page_height - bbox.top;
                copy.doctop = (self.doctop - self.top) + bbox.top;
                copy
            }
        }
    };
}

impl_bounded!(Char);
impl_bounded!(Line);
impl_bounded!(RectObject);
impl_bounded!(Curve);
impl_bounded!(ImageObject);
impl_bounded!(Annotation);
impl_bounded!(Hyperlink);

impl Bounded for Word {
    fn bbox(&self) -> BBox {
        BBox::new(self.x0, self.top, self.x1, self.bottom)
    }

    fn with_bbox(&self, bbox: BBox, _page_height: f64) -> Self {
        let mut copy = self.clone();
        copy.x0 = bbox.x0;
        copy.top = bbox.top;
        copy.x1 = bbox.x1;
        copy.bottom = bbox.bottom;
        copy.width = bbox.width();
        copy.height = bbox.height();
        copy.doctop = (self.doctop - self.top) + bbox.top;
        copy
    }
}

impl Bounded for Edge {
    fn bbox(&self) -> BBox {
        BBox::new(self.x0, self.top, self.x1, self.bottom)
    }

    fn with_bbox(&self, bbox: BBox, _page_height: f64) -> Self {
        let mut copy = self.clone();
        copy.x0 = bbox.x0;
        copy.top = bbox.top;
        copy.x1 = bbox.x1;
        copy.bottom = bbox.bottom;
        copy.width = bbox.width();
        copy.height = bbox.height();
        copy
    }
}

impl Bounded for LayoutObject {
    fn bbox(&self) -> BBox {
        BBox::new(self.x0, self.top, self.x1, self.bottom)
    }

    fn with_bbox(&self, bbox: BBox, _page_height: f64) -> Self {
        let mut copy = self.clone();
        copy.x0 = bbox.x0;
        copy.top = bbox.top;
        copy.x1 = bbox.x1;
        copy.bottom = bbox.bottom;
        copy.width = bbox.width();
        copy.height = bbox.height();
        copy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bbox_normalizes_and_intersects() {
        let bbox = BBox::new(10.0, 20.0, 0.0, 0.0).normalized();
        assert_eq!(bbox, BBox::new(0.0, 0.0, 10.0, 20.0));
        assert!(bbox.contains_point(Point::new(5.0, 5.0)));
        assert_eq!(bbox.overlap(BBox::new(5.0, 10.0, 15.0, 30.0)), Some(BBox::new(5.0, 10.0, 10.0, 20.0)));
        assert_eq!(bbox.intersection_area(BBox::new(5.0, 10.0, 15.0, 30.0)), 50.0);
    }

    #[test]
    fn object_counts_add_and_total() {
        let mut counts = ObjectCounts {
            chars: 1,
            lines: 2,
            ..ObjectCounts::default()
        };
        counts += ObjectCounts {
            rects: 3,
            hyperlinks: 4,
            ..ObjectCounts::default()
        };
        assert_eq!(counts.total(), 10);
        assert!(!counts.is_empty());
    }
}
