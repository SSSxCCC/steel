use crate::edit::Edit;
use shipyard::Component;
use steel_common::data::{Data, Limit, Value};

/// Ray tracing material, including lambertian, metal, and dielectric.
/// The albedo of lambertian/metal and the transmittance of dielectric are color.xyz * color.w,
/// the color comes from [crate::render::renderer::Renderer::color] or [crate::render::renderer2d::Renderer2D::color].
#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
pub enum Material {
    /// When a ray hits a Lambertian surface, the material reflects light uniformly in all directions.
    /// There is no sharp reflection, just a smooth, diffuse scattering.
    /// Example: Chalk, unpolished wood, painted walls, or any matte surface.
    #[default]
    Lambertian,
    /// When a ray hits a metallic surface, the material reflects most of the light at a sharp angle (specular reflection).
    /// Example: Gold, silver, copper, aluminum, iron.
    Metal {
        /// Fuzziness.
        fuzz: f32,
    },
    /// When a dielectric material is hit by a ray, the material either reflects or refracts the incoming light.
    /// Example: Glass, water, air, diamonds, or plastics.
    Dielectric {
        /// Refraction index.
        ri: f32,
    },
}

impl Material {
    /// Helper function for [Limit::Int32Enum].
    pub fn to_i32(&self) -> i32 {
        match self {
            Material::Lambertian => 0,
            Material::Metal { .. } => 1,
            Material::Dielectric { .. } => 2,
        }
    }

    /// Helper function for [Limit::Int32Enum].
    pub fn from_i32(i: i32) -> Self {
        match i {
            0 => Material::Lambertian,
            1 => Material::Metal { fuzz: 0.0 },
            2 => Material::Dielectric { ri: 1.0 },
            _ => Self::default(),
        }
    }

    /// Helper function for [Limit::Int32Enum].
    pub fn enum_vector() -> Vec<(i32, String)> {
        vec![
            (0, "Lambertian".into()),
            (1, "Metal".into()),
            (2, "Dielectric".into()),
        ]
    }
}

impl Edit for Material {
    fn name() -> &'static str {
        "Material"
    }

    fn get_data(&self) -> Data {
        let mut data = Data::new();
        data.add_value_with_limit(
            "type",
            Value::Int32(self.to_i32()),
            Limit::Int32Enum(Self::enum_vector()),
        );
        match self {
            Material::Lambertian => (),
            Material::Metal { fuzz } => data.add_value_with_limit(
                "fuzz",
                Value::Float32(*fuzz),
                Limit::Float32Range(0.0..=f32::MAX),
            ),
            Material::Dielectric { ri } => data.add_value_with_limit(
                "ri",
                Value::Float32(*ri),
                Limit::Float32Range(1.0..=f32::MAX),
            ),
        }
        data
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Int32(v)) = data.get("type") {
            if *v != self.to_i32() {
                *self = Self::from_i32(*v);
            }
        }
        match self {
            Material::Lambertian => (),
            Material::Metal { fuzz } => {
                if let Some(Value::Float32(v)) = data.get("fuzz") {
                    *fuzz = *v;
                }
            }
            Material::Dielectric { ri } => {
                if let Some(Value::Float32(v)) = data.get("ri") {
                    *ri = *v;
                }
            }
        }
    }
}
