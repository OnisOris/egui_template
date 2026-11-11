//! Fixed 128×128-cell chunks; normals sample the full field across chunk boundaries.
use super::{coordinates::to_bevy, source::TerrainPayload};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

pub const CHUNK_CELLS: u32 = 128;

pub fn chunks(data: &TerrainPayload) -> Vec<Mesh> {
    let m = &data.manifest;
    let mut result = Vec::new();
    for y in (0..m.height - 1).step_by(CHUNK_CELLS as usize) {
        for x in (0..m.width - 1).step_by(CHUNK_CELLS as usize) {
            result.push(chunk(
                data,
                x,
                y,
                CHUNK_CELLS.min(m.width - 1 - x),
                CHUNK_CELLS.min(m.height - 1 - y),
            ));
        }
    }
    result
}

pub fn chunk(data: &TerrainPayload, x: u32, y: u32, nx: u32, ny: u32) -> Mesh {
    let m = &data.manifest;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for row in y..=y + ny {
        for col in x..=x + nx {
            positions.push(to_bevy(data.position(col, row)).to_array());
            let left = col.saturating_sub(1);
            let right = (col + 1).min(m.width - 1);
            let top = row.saturating_sub(1);
            let bottom = (row + 1).min(m.height - 1);
            let dx = (f64::from(data.elevation(right, row)) - f64::from(data.elevation(left, row)))
                / (f64::from(right - left) * m.spacing_x_m);
            let dz = (f64::from(data.elevation(col, bottom)) - f64::from(data.elevation(col, top)))
                / (f64::from(bottom - top) * m.spacing_y_m);
            normals.push(
                Vec3::new(-dx as f32, 1.0, -dz as f32)
                    .normalize()
                    .to_array(),
            );
        }
    }
    let mut indices = Vec::new();
    for row in 0..ny {
        for col in 0..nx {
            let a = row * (nx + 1) + col;
            let b = a + nx + 1;
            indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub fn grid(data: &TerrainPayload) -> Mesh {
    let m = &data.manifest;
    let mut positions = Vec::new();
    let lift = (m.spacing_x_m.min(m.spacing_y_m) * 0.015).max(0.1);
    for row in (0..m.height).step_by(((m.height - 1) / 16).max(1) as usize) {
        for col in 0..m.width - 1 {
            for c in [col, col + 1] {
                let mut p = to_bevy(data.position(c, row));
                p.y += lift as f32;
                positions.push(p.to_array());
            }
        }
    }
    for col in (0..m.width).step_by(((m.width - 1) / 16).max(1) as usize) {
        for row in 0..m.height - 1 {
            for r in [row, row + 1] {
                let mut p = to_bevy(data.position(col, r));
                p.y += lift as f32;
                positions.push(p.to_array());
            }
        }
    }
    let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh
}
