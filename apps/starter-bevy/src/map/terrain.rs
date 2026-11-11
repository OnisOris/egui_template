use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

const CELLS: usize = 64;

fn elevation(u: f32, v: f32, amplitude: f32) -> f32 {
    amplitude * (0.5 + 0.25 * (u * 12.0).sin() * (v * 9.0).cos() + 0.25 * ((u + v) * 7.0).sin())
}

/// Fixed mesh budget independent of bbox size. All positions are local metres.
pub fn surface(min: Vec3, max: Vec3) -> Mesh {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let size = max - min;
    let amplitude = size.x.min(size.z) * 0.025;
    for z in 0..=CELLS {
        for x in 0..=CELLS {
            let u = x as f32 / CELLS as f32;
            let v = z as f32 / CELLS as f32;
            let y = elevation(u, v, amplitude);
            positions.push([min.x + u * size.x, y, min.z + v * size.z]);
            colors.push([
                0.15 + 0.2 * y / amplitude,
                0.34 + 0.18 * y / amplitude,
                0.25,
                1.0,
            ]);
        }
    }
    for z in 0..CELLS {
        for x in 0..CELLS {
            let a = (z * (CELLS + 1) + x) as u32;
            let b = a + (CELLS + 1) as u32;
            indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();
    mesh
}

pub fn grid(min: Vec3, max: Vec3) -> Mesh {
    let size = max - min;
    let amplitude = size.x.min(size.z) * 0.025;
    let lift = size.x.max(size.z) * 0.0005;
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    // Grid follows the same deterministic heightfield, perimeter highlighted in amber.
    for i in 0..=8 {
        let t = i as f32 / 8.0;
        let color = if i == 0 || i == 8 {
            [1.0, 0.72, 0.2, 1.0]
        } else {
            [0.35, 0.65, 0.67, 1.0]
        };
        for j in 0..CELLS {
            for swap in [false, true] {
                for k in [j, j + 1] {
                    let s = k as f32 / CELLS as f32;
                    let (u, v) = if swap { (t, s) } else { (s, t) };
                    positions.push([
                        min.x + u * size.x,
                        elevation(u, v, amplitude) + lift,
                        min.z + v * size.z,
                    ]);
                    colors.push(color);
                }
            }
        }
    }
    let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh
}

pub fn landmark_position(min: Vec3, max: Vec3, u: f32, v: f32) -> Vec3 {
    let size = max - min;
    Vec3::new(
        min.x + size.x * u,
        elevation(u, v, size.x.min(size.z) * 0.025),
        min.z + size.z * v,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_bounded_mesh_has_upward_normals() {
        let a = surface(Vec3::ZERO, Vec3::new(1000.0, 0.0, 500.0));
        let b = surface(Vec3::ZERO, Vec3::new(1000.0, 0.0, 500.0));
        assert_eq!(a.count_vertices(), 65 * 65);
        assert_eq!(a.indices().unwrap().len(), 64 * 64 * 6);
        assert_eq!(
            a.attribute(Mesh::ATTRIBUTE_POSITION),
            b.attribute(Mesh::ATTRIBUTE_POSITION)
        );
        if let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
            a.attribute(Mesh::ATTRIBUTE_NORMAL)
        {
            assert!(normals.iter().all(|n| n[1] > 0.0));
        } else {
            panic!("missing normals");
        }
    }
}
