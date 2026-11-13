//! CPU-only, chunked triangle batches; all tessellation runs on the loading worker.
use super::{clip, data::*};
use crate::map::{coordinates::local_to_wgs84, objects::terrain_elevation, source::TerrainPayload};
use bevy::{asset::RenderAssetUsages, mesh::PrimitiveTopology, prelude::*};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Triangles {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
}
impl Triangles {
    pub fn triangle(&mut self, a: Vec3, mut b: Vec3, mut c: Vec3, upward: bool) {
        let mut n = (b - a).cross(c - a);
        if n.length_squared() < 1e-10 {
            return;
        }
        if upward && n.y < 0. {
            std::mem::swap(&mut b, &mut c);
            n = -n;
        }
        self.positions
            .extend([a.to_array(), b.to_array(), c.to_array()]);
        self.normals.extend([n.normalize().to_array(); 3]);
    }
    pub fn finish(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
    }
}
pub struct Chunk {
    pub buildings: bool,
    pub flat: bool,
    pub mesh: Mesh,
}
pub fn ground(data: &TerrainPayload, p: LocalPoint2) -> Option<f32> {
    let (lon, lat) = local_to_wgs84(data.manifest.origin, p.x, p.y);
    terrain_elevation(data, lon, lat).map(|h| h as f32 - data.manifest.reference_elevation_msl)
}
fn point(p: LocalPoint2, h: f32) -> Vec3 {
    Vec3::new(p.x as f32, h, p.y as f32)
}
pub fn building(mesh: &mut Triangles, polygon: &BuildingPolygon, bottom: f32, top: f32) {
    let mut coords = Vec::new();
    let mut holes = Vec::new();
    for (i, ring) in polygon.rings.iter().enumerate() {
        if i > 0 {
            holes.push(coords.len() / 2);
        }
        for p in &ring[..ring.len() - 1] {
            coords.extend([p.x, p.y]);
        }
    }
    let Ok(indices) = earcutr::earcut(&coords, &holes, 2) else {
        return;
    };
    let vertex = |i: usize| Vec3::new(coords[i * 2] as f32, top, coords[i * 2 + 1] as f32);
    for t in indices.chunks_exact(3) {
        mesh.triangle(vertex(t[0]), vertex(t[1]), vertex(t[2]), true);
    }
    for (i, ring) in polygon.rings.iter().enumerate() {
        let area: f64 = ring
            .windows(2)
            .map(|p| p[0].x * p[1].y - p[1].x * p[0].y)
            .sum();
        for p in ring.windows(2) {
            let (a, b) = if (area > 0.) == (i == 0) {
                (p[0], p[1])
            } else {
                (p[1], p[0])
            };
            mesh.triangle(point(a, bottom), point(a, top), point(b, top), false);
            mesh.triangle(point(a, bottom), point(b, top), point(b, bottom), false);
        }
    }
}
pub fn build(data: &VectorData, terrain: &TerrainPayload, flat: bool) -> Vec<Chunk> {
    let mut uneven = 0usize;
    let mut missing = 0usize;
    let mut max_ground_span = 0f32;
    let mut batches: BTreeMap<(i32, i32, bool), Triangles> = BTreeMap::new();
    let key = |p: LocalPoint2, b| ((p.x / 512.).floor() as i32, (p.y / 512.).floor() as i32, b);
    for b in &data.buildings {
        for polygon in &b.polygons {
            let ring = &polygon.rings[0];
            // Representative ground is deliberate: rigid buildings do not deform with DEM vertices.
            let p = LocalPoint2 {
                x: ring.iter().map(|p| p.x).sum::<f64>() / ring.len() as f64,
                y: ring.iter().map(|p| p.y).sum::<f64>() / ring.len() as f64,
            };
            let Some(h) = (if flat { Some(0.) } else { ground(terrain, p) }) else {
                missing += 1;
                continue;
            };
            if !flat {
                let heights = ring
                    .iter()
                    .map(|p| ground(terrain, *p))
                    .collect::<Option<Vec<_>>>();
                let Some(heights) = heights else {
                    missing += 1;
                    continue;
                };
                let span = heights.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                    - heights.iter().copied().fold(f32::INFINITY, f32::min);
                max_ground_span = max_ground_span.max(span);
                if span > 3. {
                    uneven += 1;
                }
            }
            building(
                batches.entry(key(p, true)).or_default(),
                polygon,
                h + b.min_height_m,
                h + b.height_m,
            );
        }
    }
    let rect = clip::rectangle(data.region);
    for r in &data.roads {
        for pair in r.points.windows(2) {
            let a = pair[0];
            let b = pair[1];
            let d = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
            if d < 0.01 {
                continue;
            }
            let n = LocalPoint2 {
                x: -(b.y - a.y) / d * r.class.width() / 2.,
                y: (b.x - a.x) / d * r.class.width() / 2.,
            };
            let steps = (d / 20.).ceil() as usize;
            for i in 0..steps {
                let sample = |i| {
                    let t = i as f64 / steps as f64;
                    let p = LocalPoint2 {
                        x: a.x + (b.x - a.x) * t,
                        y: a.y + (b.y - a.y) * t,
                    };
                    let side = |s: f64| LocalPoint2 {
                        x: (p.x + n.x * s).clamp(rect.min().x, rect.max().x),
                        y: (p.y + n.y * s).clamp(rect.min().y, rect.max().y),
                    };
                    let left = side(-1.);
                    let right = side(1.);
                    let height = |p| {
                        if flat {
                            Some(0.15)
                        } else {
                            ground(terrain, p).map(|h| h + 0.15)
                        }
                    };
                    Some((point(left, height(left)?), point(right, height(right)?), p))
                };
                let (Some((a, b, p)), Some((c, d, _))) = (sample(i), sample(i + 1)) else {
                    continue;
                };
                let batch = batches.entry(key(p, false)).or_default();
                batch.triangle(a, b, c, true);
                batch.triangle(c, b, d, true);
            }
        }
    }
    if !flat {
        bevy::log::info!(
            "Building ground diagnostics: {} polygons above 3 m ground span, max={:.2} m, {} skipped without ground",
            uneven,
            max_ground_span,
            missing
        );
    }
    batches
        .into_iter()
        .filter(|(_, b)| !b.positions.is_empty())
        .map(|((_, _, buildings), b)| Chunk {
            buildings,
            flat,
            mesh: b.finish(),
        })
        .collect()
}
