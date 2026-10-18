use super::model::ModelAssets;
use crate::{asset::AssetManager, edit::Edit, shape2d::Shape2D, shape3d::Shape3D};
use glam::{vec2, vec3, Vec2, Vec3};
use obj::{Obj, TexturedVertex};
use shipyard::{Component, Unique};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};
use steel_common::{
    asset::AssetId,
    data::{Data, Limit, Value},
    platform::Platform,
};

/// Mesh component contains vertices, indices, and tex_coords to render.
/// The mesh data may come from an asset (obj or gltf file), or some predefined shapes.
/// Mesh component can be used with some other components:
/// 1. Material: defines the lighting characteristics of this object.
/// 2. Texture: defines color for each vertices by tex_coords.
#[derive(Component)]
pub enum Mesh {
    /// The asset which defines the mesh, may be obj or gltf file.
    Asset(AssetId),
    /// Predefined 2d shapes.
    Shape2D(Shape2D),
    /// Predefined 3d shapes.
    Shape3D(Shape3D),
}

impl Mesh {
    fn to_i32(&self) -> i32 {
        match self {
            Mesh::Asset(_) => 0,
            Mesh::Shape2D(_) => 1,
            Mesh::Shape3D(_) => 2,
        }
    }

    fn from_i32(i: i32) -> Self {
        match i {
            0 => Mesh::Asset(AssetId::default()),
            1 => Mesh::Shape2D(Shape2D::default()),
            2 => Mesh::Shape3D(Shape3D::default()),
            _ => Mesh::Asset(AssetId::default()),
        }
    }

    fn enum_vector() -> Vec<(i32, String)> {
        vec![
            (0, "Asset".into()),
            (1, "Shape2D".into()),
            (2, "Shape3D".into()),
        ]
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Mesh::Asset(AssetId::default())
    }
}

impl Edit for Mesh {
    fn name() -> &'static str {
        "Mesh"
    }

    fn get_data(&self, data: &mut Data) {
        data.insert_with_limit(
            "type",
            Value::Int32(self.to_i32()),
            Limit::Int32Enum(Mesh::enum_vector()),
        );
        match self {
            Mesh::Asset(asset) => {
                data.insert("asset", Value::Asset(*asset));
            }
            Mesh::Shape2D(shape) => {
                shape.get_data(data);
            }
            Mesh::Shape3D(shape) => {
                shape.get_data(data);
            }
        }
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Int32(i)) = data.get("type") {
            if *i != self.to_i32() {
                *self = Mesh::from_i32(*i);
            }
        }
        match self {
            Mesh::Asset(asset) => {
                if let Some(Value::Asset(a)) = data.get("asset") {
                    *asset = *a;
                }
            }
            Mesh::Shape2D(shape) => {
                shape.set_data(data);
            }
            Mesh::Shape3D(shape) => {
                shape.set_data(data);
            }
        }
    }
}

/// Vertex contains position, normal, and texture coordinates.
#[derive(Clone, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tex_coord: Vec2,
}

impl Eq for Vertex {}

impl std::hash::Hash for Vertex {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.position.x.to_bits().hash(state);
        self.position.y.to_bits().hash(state);
        self.position.z.to_bits().hash(state);
        self.normal.x.to_bits().hash(state);
        self.normal.y.to_bits().hash(state);
        self.normal.z.to_bits().hash(state);
        self.tex_coord.x.to_bits().hash(state);
        self.tex_coord.y.to_bits().hash(state);
    }
}

impl From<TexturedVertex> for Vertex {
    fn from(v: TexturedVertex) -> Self {
        Vertex {
            position: v.position.into(),
            normal: v.normal.into(),
            // the OBJ format assumes a coordinate system where a vertical coordinate of 0 means the bottom of the image
            tex_coord: vec2(v.texture[0], 1.0 - v.texture[1]),
        }
    }
}

/// Mesh data contains vertices and indices.
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl From<Obj<TexturedVertex>> for MeshData {
    fn from(v: Obj<TexturedVertex>) -> Self {
        MeshData {
            vertices: v.vertices.into_iter().map(|v| v.into()).collect(),
            indices: v.indices.into_iter().map(|i| i as u32).collect(),
        }
    }
}

struct MeshAsset {
    model: Arc<Obj<TexturedVertex>>,
    data: Arc<MeshData>,
}

#[derive(Unique, Default)]
/// Cache [MeshData] in assets.
pub struct MeshAssets {
    meshs: HashMap<AssetId, MeshAsset>,
}

impl MeshAssets {
    pub fn get_mesh(
        &mut self,
        asset_id: AssetId,
        model_assets: &mut ModelAssets,
        asset_manager: &mut AssetManager,
        platform: &Platform,
    ) -> Option<Arc<MeshData>> {
        if let Some(model) = model_assets.get_model(asset_id, asset_manager, platform) {
            if let Some(mesh_asset) = self.meshs.get(&asset_id) {
                if Arc::ptr_eq(&model, &mesh_asset.model) {
                    // cache is still valid
                    return Some(mesh_asset.data.clone());
                }
            }
            // cache is not valid, reload data
            let mesh_data: Arc<MeshData> = Arc::new((*model).clone().into());
            self.meshs.insert(
                asset_id,
                MeshAsset {
                    model: model.clone(),
                    data: mesh_data.clone(),
                },
            );
            return Some(mesh_data);
        }
        self.meshs.remove(&asset_id);
        None
    }
}

/// Rectangle mesh data.
pub const RECTANGLE: LazyLock<Arc<MeshData>> = LazyLock::new(|| {
    Arc::new(MeshData {
        vertices: vec![
            Vertex {
                position: vec3(-0.5, -0.5, 0.0),
                normal: vec3(0.0, 0.0, 1.0),
                tex_coord: vec2(0.0, 1.0),
            },
            Vertex {
                position: vec3(0.5, -0.5, 0.0),
                normal: vec3(0.0, 0.0, 1.0),
                tex_coord: vec2(1.0, 1.0),
            },
            Vertex {
                position: vec3(0.5, 0.5, 0.0),
                normal: vec3(0.0, 0.0, 1.0),
                tex_coord: vec2(1.0, 0.0),
            },
            Vertex {
                position: vec3(-0.5, 0.5, 0.0),
                normal: vec3(0.0, 0.0, 1.0),
                tex_coord: vec2(0.0, 0.0),
            },
        ],
        indices: vec![0, 1, 2, 2, 3, 0],
    })
});

/// Cuboid mesh data.
pub const CUBOID: LazyLock<Arc<MeshData>> = LazyLock::new(|| {
    Arc::new(MeshData {
        vertices: vec![
            // Back face (-Z)
            Vertex {
                position: vec3(-0.5, -0.5, -0.5),
                normal: vec3(0.0, 0.0, -1.0),
                tex_coord: vec2(1.0, 1.0),
            },
            Vertex {
                position: vec3(0.5, -0.5, -0.5),
                normal: vec3(0.0, 0.0, -1.0),
                tex_coord: vec2(0.0, 1.0),
            },
            Vertex {
                position: vec3(0.5, 0.5, -0.5),
                normal: vec3(0.0, 0.0, -1.0),
                tex_coord: vec2(0.0, 0.0),
            },
            Vertex {
                position: vec3(-0.5, 0.5, -0.5),
                normal: vec3(0.0, 0.0, -1.0),
                tex_coord: vec2(1.0, 0.0),
            },
            // Front face (+Z)
            Vertex {
                position: vec3(-0.5, -0.5, 0.5),
                normal: vec3(0.0, 0.0, 1.0),
                tex_coord: vec2(0.0, 1.0),
            },
            Vertex {
                position: vec3(0.5, -0.5, 0.5),
                normal: vec3(0.0, 0.0, 1.0),
                tex_coord: vec2(1.0, 1.0),
            },
            Vertex {
                position: vec3(0.5, 0.5, 0.5),
                normal: vec3(0.0, 0.0, 1.0),
                tex_coord: vec2(1.0, 0.0),
            },
            Vertex {
                position: vec3(-0.5, 0.5, 0.5),
                normal: vec3(0.0, 0.0, 1.0),
                tex_coord: vec2(0.0, 0.0),
            },
            // Left face (-X)
            Vertex {
                position: vec3(-0.5, -0.5, -0.5),
                normal: vec3(-1.0, 0.0, 0.0),
                tex_coord: vec2(0.0, 1.0),
            },
            Vertex {
                position: vec3(-0.5, -0.5, 0.5),
                normal: vec3(-1.0, 0.0, 0.0),
                tex_coord: vec2(1.0, 1.0),
            },
            Vertex {
                position: vec3(-0.5, 0.5, 0.5),
                normal: vec3(-1.0, 0.0, 0.0),
                tex_coord: vec2(1.0, 0.0),
            },
            Vertex {
                position: vec3(-0.5, 0.5, -0.5),
                normal: vec3(-1.0, 0.0, 0.0),
                tex_coord: vec2(0.0, 0.0),
            },
            // Right face (+X)
            Vertex {
                position: vec3(0.5, -0.5, -0.5),
                normal: vec3(1.0, 0.0, 0.0),
                tex_coord: vec2(1.0, 1.0),
            },
            Vertex {
                position: vec3(0.5, -0.5, 0.5),
                normal: vec3(1.0, 0.0, 0.0),
                tex_coord: vec2(0.0, 1.0),
            },
            Vertex {
                position: vec3(0.5, 0.5, 0.5),
                normal: vec3(1.0, 0.0, 0.0),
                tex_coord: vec2(0.0, 0.0),
            },
            Vertex {
                position: vec3(0.5, 0.5, -0.5),
                normal: vec3(1.0, 0.0, 0.0),
                tex_coord: vec2(1.0, 0.0),
            },
            // Top face (+Y)
            Vertex {
                position: vec3(-0.5, 0.5, -0.5),
                normal: vec3(0.0, 1.0, 0.0),
                tex_coord: vec2(0.0, 0.0),
            },
            Vertex {
                position: vec3(0.5, 0.5, -0.5),
                normal: vec3(0.0, 1.0, 0.0),
                tex_coord: vec2(1.0, 0.0),
            },
            Vertex {
                position: vec3(0.5, 0.5, 0.5),
                normal: vec3(0.0, 1.0, 0.0),
                tex_coord: vec2(1.0, 1.0),
            },
            Vertex {
                position: vec3(-0.5, 0.5, 0.5),
                normal: vec3(0.0, 1.0, 0.0),
                tex_coord: vec2(0.0, 1.0),
            },
            // Bottom face (-Y)
            Vertex {
                position: vec3(-0.5, -0.5, -0.5),
                normal: vec3(0.0, -1.0, 0.0),
                tex_coord: vec2(0.0, 1.0),
            },
            Vertex {
                position: vec3(0.5, -0.5, -0.5),
                normal: vec3(0.0, -1.0, 0.0),
                tex_coord: vec2(1.0, 1.0),
            },
            Vertex {
                position: vec3(0.5, -0.5, 0.5),
                normal: vec3(0.0, -1.0, 0.0),
                tex_coord: vec2(1.0, 0.0),
            },
            Vertex {
                position: vec3(-0.5, -0.5, 0.5),
                normal: vec3(0.0, -1.0, 0.0),
                tex_coord: vec2(0.0, 0.0),
            },
        ],
        indices: vec![
            0, 2, 1, 0, 3, 2, // Back face
            4, 5, 6, 4, 6, 7, // Front face
            8, 9, 10, 8, 10, 11, // Left face
            12, 14, 13, 12, 15, 14, // Right face
            16, 18, 17, 16, 19, 18, // Top face
            20, 21, 22, 20, 22, 23, // Bottom face
        ],
    })
});

// Sphere constants.
const SPHERE_LATITUDE_COUNT: usize = 16;
const SPHERE_LONGITUDE_COUNT: usize = 32;
const SPHERE_VERTEX_COUNT: usize = (SPHERE_LATITUDE_COUNT + 1) * (SPHERE_LONGITUDE_COUNT + 1);
const SPHERE_INDEX_COUNT: usize = SPHERE_LATITUDE_COUNT * SPHERE_LONGITUDE_COUNT * 6;

fn generate_sphere_vertices(radius: f32) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(SPHERE_VERTEX_COUNT);

    for lat in 0..=SPHERE_LATITUDE_COUNT {
        let phi = std::f32::consts::PI * lat as f32 / SPHERE_LATITUDE_COUNT as f32;
        let y = radius * phi.cos();
        let r = radius * phi.sin();

        for lon in 0..=SPHERE_LONGITUDE_COUNT {
            let theta = 2.0 * std::f32::consts::PI * lon as f32 / SPHERE_LONGITUDE_COUNT as f32;
            let x = r * theta.cos();
            let z = r * theta.sin();

            let position = Vec3::new(x, y, z);
            let normal = position.normalize();
            let tex_coord = Vec2::new(
                (1.0 - (theta / (2.0 * std::f32::consts::PI) + 0.25)) % 1.0,
                phi / std::f32::consts::PI,
            );

            vertices.push(Vertex {
                position,
                normal,
                tex_coord,
            });
        }
    }
    vertices
}

fn generate_sphere_indices() -> Vec<u32> {
    let mut indices = Vec::with_capacity(SPHERE_INDEX_COUNT);

    for lat in 0..SPHERE_LATITUDE_COUNT {
        for lon in 0..SPHERE_LONGITUDE_COUNT {
            let first = (lat * (SPHERE_LONGITUDE_COUNT + 1) + lon) as u32;
            let second = first + SPHERE_LONGITUDE_COUNT as u32 + 1;

            indices.push(first);
            indices.push(first + 1);
            indices.push(second);

            indices.push(second);
            indices.push(first + 1);
            indices.push(second + 1);
        }
    }
    indices
}

/// Sphere mesh data.
pub const SPHERE: LazyLock<Arc<MeshData>> = LazyLock::new(|| {
    let vertices = generate_sphere_vertices(0.5);
    let indices = generate_sphere_indices();

    Arc::new(MeshData { vertices, indices })
});
