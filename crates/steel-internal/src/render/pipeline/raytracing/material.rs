use crate::edit::Edit;
use glam::{Vec3, Vec4, Vec4Swizzles};
use shipyard::Component;
use steel_common::data::{Data, Limit, Value};

pub(crate) use crate::render::pipeline::raytracing::shader::raygen::EnumMaterial;

/// Ray tracing material, including lambertian, metal, and dielectric.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum Material {
    /// When a ray hits a Lambertian surface, the material reflects light uniformly in all directions.
    /// There is no sharp reflection, just a smooth, diffuse scattering.
    /// Example: Chalk, unpolished wood, painted walls, or any matte surface.
    Lambertian {
        /// Albedo color.
        color: Vec3,
    },
    /// When a ray hits a metallic surface, the material reflects most of the light at a sharp angle (specular reflection).
    /// Example: Gold, silver, copper, aluminum, iron.
    Metal {
        /// Albedo color.
        color: Vec3,
        /// Fuzziness.
        fuzz: f32,
    },
    /// When a dielectric material is hit by a ray, the material either reflects or refracts the incoming light.
    /// Example: Glass, water, air, diamonds, or plastics.
    Dielectric {
        /// Color tint.
        color: Vec3,
        /// Refraction index.
        ri: f32,
    },
    /// Emissive materials emit light instead of reflecting it.
    /// Example: Light bulbs, fire, etc.
    Emission {
        /// Emission color.
        color: Vec3,
        /// Emission intensity.
        intensity: f32,
    },
}

impl Default for Material {
    fn default() -> Self {
        Material::Lambertian { color: Vec3::ONE }
    }
}

impl Material {
    /// Get the color of the material. The rasterization rendering pipeline uses this function to
    /// make the color of the rendered object consistent with the ray tracing rendering pipeline.
    pub fn color(&self) -> Vec4 {
        match self {
            Material::Lambertian { color } => color.extend(1.0),
            Material::Metal { color, .. } => color.extend(1.0),
            Material::Dielectric { color, .. } => color.extend(1.0),
            Material::Emission { color, .. } => color.extend(1.0),
        }
    }

    /// Helper function for [Limit::Int32Enum].
    fn to_i32(&self) -> i32 {
        match self {
            Material::Lambertian { .. } => 0,
            Material::Metal { .. } => 1,
            Material::Dielectric { .. } => 2,
            Material::Emission { .. } => 3,
        }
    }

    /// Helper function for [Limit::Int32Enum].
    fn from_i32(i: i32) -> Self {
        match i {
            0 => Material::Lambertian { color: Vec3::ONE },
            1 => Material::Metal {
                color: Vec3::ONE,
                fuzz: 0.0,
            },
            2 => Material::Dielectric {
                color: Vec3::ONE,
                ri: 1.0,
            },
            3 => Material::Emission {
                color: Vec3::ONE,
                intensity: 1.0,
            },
            _ => Self::default(),
        }
    }

    /// Helper function for [Limit::Int32Enum].
    fn enum_vector() -> Vec<(i32, String)> {
        vec![
            (0, "Lambertian".into()),
            (1, "Metal".into()),
            (2, "Dielectric".into()),
            (3, "Emission".into()),
        ]
    }
}

impl Edit for Material {
    fn name() -> &'static str {
        "Material"
    }

    fn get_data(&self, data: &mut Data) {
        data.insert_with_limit(
            "type",
            Value::Int32(self.to_i32()),
            Limit::Int32Enum(Self::enum_vector()),
        );
        match self {
            Material::Lambertian { color } => {
                data.insert_with_limit("color", Value::Vec3(*color), Limit::Vec3Color);
            }
            Material::Metal { color, fuzz } => {
                data.insert_with_limit("color", Value::Vec3(*color), Limit::Vec3Color)
                    .insert_with_limit(
                        "fuzz",
                        Value::Float32(*fuzz),
                        Limit::Float32Range(0.0..=f32::MAX),
                    );
            }
            Material::Dielectric { color, ri } => {
                data.insert_with_limit("color", Value::Vec3(*color), Limit::Vec3Color)
                    .insert_with_limit(
                        "ri",
                        Value::Float32(*ri),
                        Limit::Float32Range(1.0..=f32::MAX),
                    );
            }
            Material::Emission { color, intensity } => {
                data.insert_with_limit("color", Value::Vec3(*color), Limit::Vec3Color)
                    .insert_with_limit(
                        "intensity",
                        Value::Float32(*intensity),
                        Limit::Float32Range(0.0..=f32::MAX),
                    );
            }
        }
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Int32(v)) = data.get("type") {
            if *v != self.to_i32() {
                *self = Self::from_i32(*v);
            }
        }
        match self {
            Material::Lambertian { color } => {
                if let Some(Value::Vec3(v)) = data.get("color") {
                    *color = *v;
                }
            }
            Material::Metal { color, fuzz } => {
                if let Some(Value::Vec3(v)) = data.get("color") {
                    *color = *v;
                }
                if let Some(Value::Float32(v)) = data.get("fuzz") {
                    *fuzz = *v;
                }
            }
            Material::Dielectric { color, ri } => {
                if let Some(Value::Vec3(v)) = data.get("color") {
                    *color = *v;
                }
                if let Some(Value::Float32(v)) = data.get("ri") {
                    *ri = *v;
                }
            }
            Material::Emission { color, intensity } => {
                if let Some(Value::Vec3(v)) = data.get("color") {
                    *color = *v;
                }
                if let Some(Value::Float32(v)) = data.get("intensity") {
                    *intensity = *v;
                }
            }
        }
    }
}

impl EnumMaterial {
    pub fn new_lambertian(color: Vec3) -> Self {
        Self {
            data: [color.x, color.y, color.z, 0.0],
            t: 0,
        }
    }

    pub fn new_metal(color: Vec3, fuzz: f32) -> Self {
        Self {
            data: [color.x, color.y, color.z, fuzz],
            t: 1,
        }
    }

    pub fn new_dielectric(color: Vec3, ri: f32) -> Self {
        Self {
            data: [color.x, color.y, color.z, ri],
            t: 2,
        }
    }

    pub fn new_emission(color: Vec3, intensity: f32) -> Self {
        Self {
            data: [color.x, color.y, color.z, intensity],
            t: 3,
        }
    }

    pub fn from_material(material: Material, color: Vec4) -> Self {
        let color_factor = color.xyz() * color.w;
        match material {
            Material::Lambertian { color } => Self::new_lambertian(color_factor * color),
            Material::Metal { color, fuzz } => Self::new_metal(color_factor * color, fuzz),
            Material::Dielectric { color, ri } => Self::new_dielectric(color_factor * color, ri),
            Material::Emission { color, intensity } => {
                Self::new_emission(color_factor * color, intensity)
            }
        }
    }
}
