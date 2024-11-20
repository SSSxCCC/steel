use crate::{
    edit::Edit, hierarchy::Parent, render::canvas::Canvas, shape2d::Shape2D, transform::Transform,
};
use glam::{Affine3A, Vec3, Vec4};
use parry2d::shape::ShapeType;
use shipyard::{Component, IntoIter, IntoWithId, UniqueViewMut, View};
use std::collections::HashMap;
use steel_common::{
    asset::AssetId,
    data::{Data, Limit, Value},
};

/// The 2d render object used by [Renderer2D], may be a 2d shape or 2d texture.
#[derive(Debug)]
pub enum RenderObject2D {
    Shape(Shape2D),
    Texture(AssetId),
}

impl RenderObject2D {
    pub fn to_i32(&self) -> i32 {
        match self {
            Self::Shape(_) => 0,
            Self::Texture(_) => 1,
        }
    }

    pub fn from_i32(i: i32) -> Self {
        match i {
            0 => Self::Shape(Shape2D::default()),
            1 => Self::Texture(AssetId::default()),
            _ => Self::Shape(Shape2D::default()),
        }
    }

    pub fn enum_vector() -> Vec<(i32, String)> {
        vec![(0, "Shape".into()), (1, "Texture".into())]
    }
}

/// Renderer2D component is used to draw a 2D shape or 2D texture on an entity.
#[derive(Component, Debug)]
pub struct Renderer2D {
    pub object: RenderObject2D,
    pub color: Vec4,
}

impl Default for Renderer2D {
    fn default() -> Self {
        Self {
            object: RenderObject2D::Shape(Shape2D::default()),
            color: Vec4::ONE, /* white */
        }
    }
}

impl Edit for Renderer2D {
    fn name() -> &'static str {
        "Renderer2D"
    }

    fn get_data(&self) -> Data {
        let mut data = Data::new();
        data.add_value_with_limit(
            "render_object",
            Value::Int32(self.object.to_i32()),
            Limit::Int32Enum(RenderObject2D::enum_vector()),
        );
        match &self.object {
            RenderObject2D::Shape(shape) => shape.get_data(&mut data),
            RenderObject2D::Texture(asset_id) => {
                data.add_value("unnamed-0", Value::Asset(*asset_id))
            }
        }
        data.insert_with_limit("color", Value::Vec4(self.color), Limit::Vec4Color)
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Int32(render_object)) = data.get("render_object") {
            if self.object.to_i32() != *render_object {
                self.object = RenderObject2D::from_i32(*render_object);
            }
            match &mut self.object {
                RenderObject2D::Shape(shape) => shape.set_data(data),
                RenderObject2D::Texture(asset_id) => {
                    if let Some(Value::Asset(a)) = data.get("unnamed-0") {
                        *asset_id = *a;
                    }
                }
            }
        }
        if let Some(Value::Vec4(v)) = data.get("color") {
            self.color = *v
        }
    }
}

/// Add drawing data to the [Canvas] unique according to the [Renderer2D] components.
pub fn renderer2d_to_canvas_system(
    renderer2d: View<Renderer2D>,
    transforms: View<Transform>,
    children: View<Parent>,
    mut canvas: UniqueViewMut<Canvas>,
) {
    let mut model_cache = Some(HashMap::new());
    let mut scale_cache = Some(HashMap::new());
    for (eid, (renderer2d, _)) in (&renderer2d, &transforms).iter().with_id() {
        let scale =
            Transform::entity_final_scale(eid, &children, &transforms, &mut scale_cache).unwrap();
        let model_without_scale = Transform::entity_final_model_without_scale(
            eid,
            &children,
            &transforms,
            &mut model_cache,
        )
        .unwrap();
        let model = model_without_scale * Affine3A::from_scale(scale);
        match &renderer2d.object {
            RenderObject2D::Shape(shape) => match shape.shape_type() {
                ShapeType::Ball => {
                    let scale = std::cmp::max_by(scale.x.abs(), scale.y.abs(), |x, y| {
                        x.partial_cmp(y).unwrap()
                    });
                    let radius = shape.as_ball().unwrap().radius * scale;
                    let (_, rotation, position) =
                        model_without_scale.to_scale_rotation_translation();
                    canvas.circle(position, rotation, radius, renderer2d.color, eid);
                }
                ShapeType::Cuboid => {
                    let shape = shape.as_cuboid().unwrap();
                    let (half_width, half_height) = (shape.half_extents.x, shape.half_extents.y);
                    canvas.rectangle(
                        model.transform_point3(Vec3::new(-half_width, -half_height, 0.0)),
                        model.transform_point3(Vec3::new(-half_width, half_height, 0.0)),
                        model.transform_point3(Vec3::new(half_width, half_height, 0.0)),
                        model.transform_point3(Vec3::new(half_width, -half_height, 0.0)),
                        renderer2d.color,
                        eid,
                    );
                }
                _ => (),
            },
            RenderObject2D::Texture(asset) => canvas.texture(*asset, renderer2d.color, model, eid),
        }
    }
}
