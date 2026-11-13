use super::{clip, data::*};
use crate::map::{builder::coverage::Tile, coordinates::wgs84_to_local};
use geo::{Geometry, MapCoords};
use mvt_reader::{Reader, feature::Value};
use starter_core::MapRegion;
use std::collections::{HashMap, HashSet};

fn number(p: &HashMap<String, Value>, key: &str) -> Option<f32> {
    match p.get(key)? {
        Value::Float(n) => Some(*n),
        Value::Double(n) => Some(*n as f32),
        Value::Int(n) | Value::SInt(n) => Some(*n as f32),
        Value::UInt(n) => Some(*n as f32),
        Value::String(n) => n.parse().ok(),
        _ => None,
    }
    .filter(|n| n.is_finite())
}
fn string<'a>(p: &'a HashMap<String, Value>, key: &str) -> Option<&'a str> {
    match p.get(key)? {
        Value::String(s) => Some(s),
        _ => None,
    }
}
fn class(s: &str) -> Option<RoadClass> {
    Some(match s {
        "motorway" => RoadClass::Motorway,
        "trunk" => RoadClass::Trunk,
        "primary" => RoadClass::Primary,
        "secondary" => RoadClass::Secondary,
        "tertiary" => RoadClass::Tertiary,
        "minor" => RoadClass::Minor,
        "service" => RoadClass::Service,
        _ => return None,
    })
}
pub fn extract(bytes: Vec<u8>, tile: Tile, data: &mut VectorData) -> Result<(), String> {
    let reader = Reader::new(bytes).map_err(|e| e.to_string())?;
    let rect = clip::rectangle(data.region);
    for layer in reader.get_layer_metadata().map_err(|e| e.to_string())? {
        if !matches!(layer.name.as_str(), "building" | "transportation") {
            continue;
        }
        if layer.extent == 0 || layer.feature_count > MAX_FEATURES {
            return Err("MVT layer limit exceeded".into());
        }
        for (i, f) in reader
            .get_features_as::<f64>(layer.layer_index)
            .map_err(|e| e.to_string())?
            .into_iter()
            .enumerate()
        {
            let p = f.properties.unwrap_or_default();
            let id = format!(
                "{}-{}-{}-{}",
                tile.z,
                tile.x,
                tile.y,
                f.id.unwrap_or(i as u64)
            );
            let geometry = f.geometry.map_coords(|c| {
                let n = 2f64.powi(i32::from(tile.z));
                let lon = (f64::from(tile.x) + c.x / f64::from(layer.extent)) / n * 360. - 180.;
                let lat = (std::f64::consts::PI
                    * (1. - 2. * (f64::from(tile.y) + c.y / f64::from(layer.extent)) / n))
                    .sinh()
                    .atan()
                    .to_degrees();
                let p = wgs84_to_local(data.region.origin, lon, lat);
                geo::Coord { x: p[0], y: p[1] }
            });
            if layer.name == "transportation" {
                if string(&p, "brunnel") == Some("tunnel") {
                    continue;
                }
                let Some(class) = string(&p, "class").and_then(class) else {
                    continue;
                };
                let lines = match geometry {
                    Geometry::LineString(l) => vec![l],
                    Geometry::MultiLineString(l) => l.0,
                    _ => continue,
                };
                for (part, line) in lines.into_iter().enumerate() {
                    let points: Vec<_> = line
                        .0
                        .iter()
                        .map(|p| LocalPoint2 { x: p.x, y: p.y })
                        .collect();
                    for (fragment, points) in clip::roads(&points, rect).into_iter().enumerate() {
                        data.roads.push(RoadFeature {
                            id: format!("{id}-{part}-{fragment}"),
                            class,
                            points,
                        });
                    }
                }
            } else {
                if matches!(p.get("hide_3d"), Some(Value::Bool(true))) {
                    continue;
                }
                // OpenMapTiles frequently lacks surveyed heights: 12 m is an approximation.
                let height_m = number(&p, "render_height")
                    .or_else(|| number(&p, "height"))
                    .unwrap_or(12.);
                let min_height_m = number(&p, "render_min_height")
                    .or_else(|| number(&p, "min_height"))
                    .unwrap_or(0.);
                if !(0. ..=2000.).contains(&height_m)
                    || min_height_m < 0.
                    || height_m <= min_height_m
                {
                    continue;
                }
                let polygons = match geometry {
                    Geometry::Polygon(p) => vec![p],
                    Geometry::MultiPolygon(p) => p.0,
                    _ => continue,
                };
                let polygons = polygons
                    .into_iter()
                    .flat_map(|p| {
                        let rings = std::iter::once(p.exterior())
                            .chain(p.interiors())
                            .map(|r| r.0.iter().map(|p| LocalPoint2 { x: p.x, y: p.y }).collect())
                            .collect();
                        clip::buildings(&BuildingPolygon { rings }, rect)
                    })
                    .collect::<Vec<_>>();
                if !polygons.is_empty() {
                    data.buildings.push(BuildingFeature {
                        id,
                        polygons,
                        height_m,
                        min_height_m,
                    });
                }
            }
            if data.roads.len() + data.buildings.len() > MAX_FEATURES {
                return Err("Vector feature limit exceeded".into());
            }
        }
    }
    Ok(())
}
pub fn deduplicate(data: &mut VectorData) {
    // Buffer copies in neighbouring MVT tiles have distinct IDs: compare clipped geometry.
    let mut seen = HashSet::new();
    data.buildings.retain(|b| {
        seen.insert(
            bincode::serde::encode_to_vec(
                (&b.polygons, b.height_m, b.min_height_m),
                bincode::config::standard(),
            )
            .unwrap(),
        )
    });
    seen.clear();
    data.roads.retain(|r| {
        seen.insert(
            bincode::serde::encode_to_vec((&r.class, &r.points), bincode::config::standard())
                .unwrap(),
        )
    });
}
pub fn empty(region: MapRegion, id: String) -> VectorData {
    VectorData {
        region,
        source: "OpenFreeMap".into(),
        attribution: "© OpenFreeMap © OpenMapTiles © OpenStreetMap contributors".into(),
        package_id: id,
        fingerprint: blake3::hash(&[]),
        roads: Vec::new(),
        buildings: Vec::new(),
    }
}
