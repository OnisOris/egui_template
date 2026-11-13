use super::data::*;
use crate::map::coordinates::wgs84_to_local;
use geo::{Area, BooleanOps, Coord, LineString, MultiLineString, Polygon, Rect, Validation};
use starter_core::MapRegion;

pub fn rectangle(region: MapRegion) -> Rect<f64> {
    let b = region.bounds;
    let a = wgs84_to_local(region.origin, b.west, b.north);
    let z = wgs84_to_local(region.origin, b.east, b.south);
    Rect::new(Coord { x: a[0], y: a[1] }, Coord { x: z[0], y: z[1] })
}
pub fn line(points: &[LocalPoint2]) -> LineString<f64> {
    LineString::new(points.iter().map(|p| Coord { x: p.x, y: p.y }).collect())
}
pub fn polygon(p: &BuildingPolygon) -> Option<Polygon<f64>> {
    let (outer, holes) = p.rings.split_first()?;
    let polygon = Polygon::new(line(outer), holes.iter().map(|r| line(r)).collect());
    (polygon.is_valid() && polygon.unsigned_area() > 0.01).then_some(polygon)
}
fn points(line: &LineString<f64>, rect: Rect<f64>) -> Vec<LocalPoint2> {
    line.0
        .iter()
        .map(|p| LocalPoint2 {
            x: p.x.clamp(rect.min().x, rect.max().x),
            y: p.y.clamp(rect.min().y, rect.max().y),
        })
        .collect()
}
pub fn buildings(p: &BuildingPolygon, rect: Rect<f64>) -> Vec<BuildingPolygon> {
    let Some(p) = polygon(p) else {
        return Vec::new();
    };
    p.intersection(&rect.to_polygon())
        .0
        .iter()
        .filter(|p| p.unsigned_area() > 0.01)
        .map(|p| BuildingPolygon {
            rings: std::iter::once(p.exterior())
                .chain(p.interiors())
                .map(|l| points(l, rect))
                .collect(),
        })
        .collect()
}
pub fn roads(p: &[LocalPoint2], rect: Rect<f64>) -> Vec<Vec<LocalPoint2>> {
    rect.to_polygon()
        .clip(&MultiLineString(vec![line(p)]), false)
        .0
        .iter()
        .filter(|l| l.0.len() >= 2)
        .map(|l| points(l, rect))
        .collect()
}
pub fn validate(data: &VectorData) -> Result<(), String> {
    MapRegion::new(data.region.bounds, data.region.origin).map_err(String::from)?;
    if data.roads.len() + data.buildings.len() > MAX_FEATURES {
        return Err("Vector feature limit exceeded".into());
    }
    let rect = rectangle(data.region);
    let mut count = 0;
    let mut valid_points = |points: &[LocalPoint2]| {
        count += points.len();
        points.iter().all(|p| {
            p.x.is_finite()
                && p.y.is_finite()
                && p.x >= rect.min().x - 0.05
                && p.x <= rect.max().x + 0.05
                && p.y >= rect.min().y - 0.05
                && p.y <= rect.max().y + 0.05
        })
    };
    for r in &data.roads {
        if r.id.len() > 256 || r.points.len() < 2 || !valid_points(&r.points) {
            return Err("Invalid road coordinates or ID".into());
        }
    }
    for b in &data.buildings {
        if b.id.len() > 256
            || b.polygons.is_empty()
            || !b.height_m.is_finite()
            || !b.min_height_m.is_finite()
            || b.min_height_m < 0.
            || b.height_m <= b.min_height_m
            || b.height_m > 2000.
        {
            return Err("Invalid building height, polygons or ID".into());
        }
        for p in &b.polygons {
            if p.rings
                .iter()
                .any(|r| r.len() < 4 || r.first() != r.last() || !valid_points(r))
                || polygon(p).is_none()
            {
                return Err("Invalid building polygon".into());
            }
        }
    }
    if count > MAX_POINTS {
        return Err("Vector coordinate limit exceeded".into());
    }
    Ok(())
}
