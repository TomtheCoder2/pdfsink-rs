use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
        self.x0 <= self.x1 && self.top <= self.bottom
    }

    pub fn overlaps(self, other: Self) -> bool {
        !(self.x1 < other.x0 || self.x0 > other.x1 || self.bottom < other.top || self.top > other.bottom)
    }

    pub fn contains_bbox(self, other: Self) -> bool {
        self.x0 <= other.x0
            && self.top <= other.top
            && self.x1 >= other.x1
            && self.bottom >= other.bottom
    }

    pub fn overlap(self, other: Self) -> Option<Self> {
        let x0 = self.x0.max(other.x0);
        let top = self.top.max(other.top);
        let x1 = self.x1.min(other.x1);
        let bottom = self.bottom.min(other.bottom);
        if x1 >= x0 && bottom >= top && ((x1 - x0) + (bottom - top) > 0.0) {
            Some(Self::new(x0, top, x1, bottom))
        } else {
            None
        }
    }

    pub fn translate(self, dx: f64, dy: f64) -> Self {
        Self::new(self.x0 + dx, self.top + dy, self.x1 + dx, self.bottom + dy)
    }

    pub fn as_tuple(self) -> (f64, f64, f64, f64) {
        (self.x0, self.top, self.x1, self.bottom)
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Annotation {
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
pub struct Page {
    pub page_number: usize,
    pub rotation: i32,
    pub width: f64,
    pub height: f64,
    pub bbox: BBox,
    pub doctop_offset: f64,
    pub chars: Vec<Char>,
    pub lines: Vec<Line>,
    pub rects: Vec<RectObject>,
    pub curves: Vec<Curve>,
    pub images: Vec<ImageObject>,
    pub annots: Vec<Annotation>,
    pub hyperlinks: Vec<Hyperlink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PdfDocument {
    pub path: PathBuf,
    pub pages: Vec<Page>,
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
