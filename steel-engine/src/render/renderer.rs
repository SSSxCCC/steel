use crate::{
    edit::Edit, hierarchy::Parent, render::canvas::Canvas, shape::Shape, transform::Transform,
};
use glam::{Affine3A, Vec3, Vec4};
use parry3d::shape::ShapeType;
use shipyard::{Component, IntoIter, IntoWithId, UniqueViewMut, View};
use std::collections::HashMap;
use steel_common::{
    asset::AssetId,
    data::{Data, Limit, Value},
};

/// The 3d render object used by [Renderer], may be a 3d shape or 3d model.
#[derive(Debug)]
pub enum RenderObject {
    Shape(Shape),
    Model(AssetId),
}

impl RenderObject {
    pub fn to_i32(&self) -> i32 {
        match self {
            Self::Shape(_) => 0,
            Self::Model(_) => 1,
        }
    }

    pub fn from_i32(i: i32) -> Self {
        match i {
            0 => Self::Shape(Shape::default()),
            1 => Self::Model(AssetId::default()),
            _ => Self::Shape(Shape::default()),
        }
    }

    pub fn enum_vector() -> Vec<(i32, String)> {
        vec![(0, "Shape".into()), (1, "Model".into())]
    }
}

/// Renderer component is used to draw a 3D shape or 3D model on an entity.
#[derive(Component, Debug)]
pub struct Renderer {
    pub object: RenderObject,
    pub color: Vec4,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            object: RenderObject::Shape(Shape::default()),
            color: Vec4::ONE, /* white */
        }
    }
}

impl Edit for Renderer {
    fn name() -> &'static str {
        "Renderer"
    }

    fn get_data(&self) -> Data {
        let mut data = Data::new();
        data.add_value_with_limit(
            "render_object",
            Value::Int32(self.object.to_i32()),
            Limit::Int32Enum(RenderObject::enum_vector()),
        );
        match &self.object {
            RenderObject::Shape(shape) => shape.get_data(&mut data),
            RenderObject::Model(asset_id) => data.add_value("unnamed-0", Value::Asset(*asset_id)),
        }
        data.insert_with_limit("color", Value::Vec4(self.color), Limit::Vec4Color)
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Int32(render_object)) = data.get("render_object") {
            if self.object.to_i32() != *render_object {
                self.object = RenderObject::from_i32(*render_object);
            }
            match &mut self.object {
                RenderObject::Shape(shape) => shape.set_data(data),
                RenderObject::Model(asset_id) => {
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

/// Add drawing data to the [Canvas] unique according to the [Renderer] components.
pub fn renderer_to_canvas_system(
    renderer: View<Renderer>,
    transforms: View<Transform>,
    children: View<Parent>,
    mut canvas: UniqueViewMut<Canvas>,
) {
    let mut model_cache = Some(HashMap::new());
    let mut scale_cache = Some(HashMap::new());
    for (eid, (renderer, _)) in (&renderer, &transforms).iter().with_id() {
        let scale =
            Transform::entity_final_scale(eid, &children, &transforms, &mut scale_cache).unwrap();
        let model_without_scale = Transform::entity_final_model_without_scale(
            eid,
            &children,
            &transforms,
            &mut model_cache,
        )
        .unwrap();
        match &renderer.object {
            RenderObject::Shape(shape) => match shape.shape_type() {
                ShapeType::Ball => {
                    let scale = shape.as_ball().unwrap().radius / 0.5
                        * [scale.x.abs(), scale.y.abs(), scale.z.abs()]
                            .into_iter()
                            .fold(f32::NEG_INFINITY, |max, val| max.max(val));
                    let model =
                        model_without_scale * Affine3A::from_scale(Vec3::new(scale, scale, scale));
                    canvas.sphere(model, renderer.color, eid);
                }
                ShapeType::Cuboid => {
                    let shape = shape.as_cuboid().unwrap();
                    let scale = Vec3::new(
                        scale.x * shape.half_extents.x * 2.0,
                        scale.y * shape.half_extents.y * 2.0,
                        scale.z * shape.half_extents.z * 2.0,
                    );
                    let model = model_without_scale * Affine3A::from_scale(scale);
                    canvas.cuboid(model, renderer.color, eid);
                }
                _ => (),
            },
            RenderObject::Model(_) => (),
        }
    }
}
