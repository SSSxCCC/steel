use glam::Vec3;
use std::sync::LazyLock;

pub const RECTANGLE_VERTICES: [Vec3; 4] = [
    Vec3::new(-0.5, -0.5, 0.0), // Bottom-left corner
    Vec3::new(0.5, -0.5, 0.0),  // Bottom-right corner
    Vec3::new(0.5, 0.5, 0.0),   // Top-right corner
    Vec3::new(-0.5, 0.5, 0.0),  // Top-left corner
];

pub const RECTANGLE_INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

pub const CUBOID_VERTICES: [Vec3; 8] = [
    Vec3::new(-0.5, -0.5, -0.5), // Bottom-left-back
    Vec3::new(-0.5, 0.5, -0.5),  // Top-left-back
    Vec3::new(0.5, 0.5, -0.5),   // Top-right-back
    Vec3::new(0.5, -0.5, -0.5),  // Bottom-right-back
    Vec3::new(-0.5, -0.5, 0.5),  // Bottom-left-front
    Vec3::new(-0.5, 0.5, 0.5),   // Top-left-front
    Vec3::new(0.5, 0.5, 0.5),    // Top-right-front
    Vec3::new(0.5, -0.5, 0.5),   // Bottom-right-front
];

pub const CUBOID_INDICES: [u16; 36] = [
    0, 2, 1, 0, 3, 2, // Back face
    4, 5, 6, 4, 6, 7, // Front face
    0, 1, 5, 0, 5, 4, // Left face
    3, 7, 6, 3, 6, 2, // Right face
    1, 2, 6, 1, 6, 5, // Top face
    0, 4, 7, 0, 7, 3, // Bottom face
];

pub const SPHERE_VERTICES: LazyLock<[Vec3; SPHERE_VERTEX_COUNT]> =
    LazyLock::new(|| generate_sphere_vertices(0.5));
pub const SPHERE_INDICES: [u16; SPHERE_INDEX_COUNT] = generate_sphere_indices();
const SPHERE_LATITUDE_COUNT: usize = 16;
const SPHERE_LONGITUDE_COUNT: usize = 32;
const SPHERE_VERTEX_COUNT: usize = (SPHERE_LATITUDE_COUNT + 1) * (SPHERE_LONGITUDE_COUNT + 1);
const SPHERE_INDEX_COUNT: usize = SPHERE_LATITUDE_COUNT * SPHERE_LONGITUDE_COUNT * 6;

/// This function need to make floating point arithmetic so that it can not be const.
fn generate_sphere_vertices(radius: f32) -> [Vec3; SPHERE_VERTEX_COUNT] {
    let mut vertices = [Vec3::ZERO; SPHERE_VERTEX_COUNT];
    let mut idx = 0;

    let mut lat = 0;
    while lat <= SPHERE_LATITUDE_COUNT {
        let phi = std::f32::consts::PI * lat as f32 / SPHERE_LATITUDE_COUNT as f32;
        let y = radius * phi.cos();
        let r = radius * phi.sin();

        let mut lon = 0;
        while lon <= SPHERE_LONGITUDE_COUNT {
            let theta = 2.0 * std::f32::consts::PI * lon as f32 / SPHERE_LONGITUDE_COUNT as f32;
            let x = r * theta.cos();
            let z = r * theta.sin();

            vertices[idx] = Vec3::new(x, y, z);
            idx += 1;
            lon += 1;
        }
        lat += 1;
    }
    vertices
}

const fn generate_sphere_indices() -> [u16; SPHERE_INDEX_COUNT] {
    let mut indices = [0; SPHERE_INDEX_COUNT];
    let mut idx = 0;

    let mut lat = 0;
    while lat < SPHERE_LATITUDE_COUNT {
        let mut lon = 0;
        while lon < SPHERE_LONGITUDE_COUNT {
            let first = (lat * (SPHERE_LONGITUDE_COUNT + 1) + lon) as u16;
            let second = first + SPHERE_LONGITUDE_COUNT as u16 + 1;

            indices[idx] = first;
            indices[idx + 1] = second;
            indices[idx + 2] = first + 1;

            indices[idx + 3] = second;
            indices[idx + 4] = second + 1;
            indices[idx + 5] = first + 1;

            idx += 6;
            lon += 1;
        }
        lat += 1;
    }
    indices
}
