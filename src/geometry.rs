use crate::error::{Error, Result};
use crate::types::{BBox, Bounded, Edge, Orientation, Point, RectObject};

pub fn objects_to_bbox<T: Bounded>(objects: &[T]) -> Option<BBox> {
    if objects.is_empty() {
        return None;
    }
    let mut x0 = f64::INFINITY;
    let mut top = f64::INFINITY;
    let mut x1 = f64::NEG_INFINITY;
    let mut bottom = f64::NEG_INFINITY;

    for obj in objects {
        let bbox = obj.bbox();
        x0 = x0.min(bbox.x0);
        top = top.min(bbox.top);
        x1 = x1.max(bbox.x1);
        bottom = bottom.max(bbox.bottom);
    }

    Some(BBox::new(x0, top, x1, bottom))
}

pub fn bbox_from_points(points: &[Point]) -> Option<BBox> {
    if points.is_empty() {
        return None;
    }
    let mut x0 = f64::INFINITY;
    let mut top = f64::INFINITY;
    let mut x1 = f64::NEG_INFINITY;
    let mut bottom = f64::NEG_INFINITY;
    for point in points {
        x0 = x0.min(point.x);
        top = top.min(point.y);
        x1 = x1.max(point.x);
        bottom = bottom.max(point.y);
    }
    Some(BBox::new(x0, top, x1, bottom))
}

pub fn calculate_area(bbox: BBox) -> Result<f64> {
    if !bbox.is_valid() {
        return Err(Error::InvalidBBox(format!("negative bbox dimensions: {:?}", bbox)));
    }
    Ok(bbox.area())
}

pub fn test_proposed_bbox(bbox: BBox, parent_bbox: BBox) -> Result<()> {
    let bbox_area = calculate_area(bbox)?;
    if bbox_area == 0.0 {
        return Err(Error::InvalidBBox(format!("zero-area bbox: {:?}", bbox)));
    }
    let overlap = bbox
        .overlap(parent_bbox)
        .ok_or_else(|| Error::InvalidBBox(format!("bbox {:?} is entirely outside {:?}", bbox, parent_bbox)))?;
    let overlap_area = calculate_area(overlap)?;
    if overlap_area < bbox_area {
        return Err(Error::InvalidBBox(format!(
            "bbox {:?} is not fully within parent bbox {:?}",
            bbox, parent_bbox
        )));
    }
    Ok(())
}

pub fn crop_objects<T: Bounded>(objects: &[T], bbox: BBox, page_height: f64) -> Vec<T> {
    objects
        .iter()
        .filter_map(|obj| obj.bbox().overlap(bbox).map(|clipped| obj.with_bbox(clipped, page_height)))
        .collect()
}

pub fn within_objects<T: Bounded>(objects: &[T], bbox: BBox) -> Vec<T> {
    objects
        .iter()
        .filter(|obj| bbox.contains_bbox(obj.bbox()))
        .cloned()
        .collect()
}

pub fn outside_objects<T: Bounded>(objects: &[T], bbox: BBox) -> Vec<T> {
    objects
        .iter()
        .filter(|obj| obj.bbox().overlap(bbox).is_none())
        .cloned()
        .collect()
}

pub fn snap_edges(edges: &[Edge], x_tolerance: f64, y_tolerance: f64) -> Vec<Edge> {
    let mut vertical: Vec<Edge> = edges
        .iter()
        .filter(|edge| edge.orientation == Orientation::Vertical)
        .cloned()
        .collect();
    let mut horizontal: Vec<Edge> = edges
        .iter()
        .filter(|edge| edge.orientation == Orientation::Horizontal)
        .cloned()
        .collect();

    if !vertical.is_empty() && x_tolerance > 0.0 {
        vertical.sort_by(|a, b| a.x0.total_cmp(&b.x0));
        vertical = snap_objects(&vertical, |edge| edge.x0, x_tolerance, true);
    }

    if !horizontal.is_empty() && y_tolerance > 0.0 {
        horizontal.sort_by(|a, b| a.top.total_cmp(&b.top));
        horizontal = snap_objects(&horizontal, |edge| edge.top, y_tolerance, false);
    }

    vertical.extend(horizontal);
    vertical
}

pub fn snap_objects<F>(objects: &[Edge], key: F, tolerance: f64, vertical: bool) -> Vec<Edge>
where
    F: Fn(&Edge) -> f64,
{
    if objects.is_empty() {
        return Vec::new();
    }

    let mut groups: Vec<Vec<Edge>> = Vec::new();
    let mut sorted = objects.to_vec();
    sorted.sort_by(|a, b| key(a).total_cmp(&key(b)));

    let mut current = vec![sorted[0].clone()];
    let mut last = key(&sorted[0]);
    for edge in sorted.into_iter().skip(1) {
        let value = key(&edge);
        if value <= last + tolerance {
            current.push(edge);
        } else {
            groups.push(current);
            current = vec![edge];
        }
        last = value;
    }
    groups.push(current);

    let mut out = Vec::new();
    for group in groups {
        let avg = group.iter().map(|edge| key(edge)).sum::<f64>() / group.len() as f64;
        for edge in group {
            let mut moved = edge.clone();
            if vertical {
                moved.x1 += avg - moved.x0;
                moved.x0 = avg;
                moved.width = moved.x1 - moved.x0;
            } else {
                moved.bottom += avg - moved.top;
                moved.top = avg;
                moved.height = moved.bottom - moved.top;
            }
            out.push(moved);
        }
    }
    out
}

pub fn rect_to_edges(rect: &RectObject) -> Vec<Edge> {
    vec![
        Edge {
            x0: rect.x0,
            top: rect.top,
            x1: rect.x1,
            bottom: rect.top,
            width: rect.width,
            height: 0.0,
            orientation: Orientation::Horizontal,
            object_type: "rect_edge".to_string(),
        },
        Edge {
            x0: rect.x0,
            top: rect.bottom,
            x1: rect.x1,
            bottom: rect.bottom,
            width: rect.width,
            height: 0.0,
            orientation: Orientation::Horizontal,
            object_type: "rect_edge".to_string(),
        },
        Edge {
            x0: rect.x0,
            top: rect.top,
            x1: rect.x0,
            bottom: rect.bottom,
            width: 0.0,
            height: rect.height,
            orientation: Orientation::Vertical,
            object_type: "rect_edge".to_string(),
        },
        Edge {
            x0: rect.x1,
            top: rect.top,
            x1: rect.x1,
            bottom: rect.bottom,
            width: 0.0,
            height: rect.height,
            orientation: Orientation::Vertical,
            object_type: "rect_edge".to_string(),
        },
    ]
}
