use crate::{
    edit::Edit, hierarchy::Parent, render::canvas::Canvas, shape::ShapeWrapper,
    transform::Transform,
};
use glam::{Affine3A, Vec3, Vec4};
use parry2d::shape::{ShapeType, SharedShape};
use shipyard::{Component, IntoIter, IntoWithId, UniqueViewMut, View};
use std::collections::HashMap;
use steel_common::data::{Data, Limit, Value};

/// Renderer2D component is used to draw a 2D shape on an entity.
#[derive(Component, Debug)]
pub struct Renderer2D {
    pub shape: ShapeWrapper,
    pub color: Vec4,
}

impl Default for Renderer2D {
    fn default() -> Self {
        Self {
            shape: ShapeWrapper(SharedShape::cuboid(0.5, 0.5)),
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
        self.shape.get_data(&mut data);
        data.insert_with_limit("color", Value::Vec4(self.color), Limit::Vec4Color)
    }

    fn set_data(&mut self, data: &Data) {
        self.shape.set_data(data);
        if let Some(Value::Vec4(v)) = data.get("color") {
            self.color = *v
        }
    }
}

/// Add drawing data to the Canvas unique according to the Renderer2D components.
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
        match renderer2d.shape.shape_type() {
            ShapeType::Ball => {
                let scale = std::cmp::max_by(scale.x.abs(), scale.y.abs(), |x, y| {
                    x.partial_cmp(y).unwrap()
                });
                let radius = renderer2d.shape.as_ball().unwrap().radius * scale;
                let (_, rotation, position) = model_without_scale.to_scale_rotation_translation();
                canvas.circle(position, rotation, radius, renderer2d.color, eid);
            }
            ShapeType::Cuboid => {
                let shape = renderer2d.shape.as_cuboid().unwrap();
                let (half_width, half_height) = (shape.half_extents.x, shape.half_extents.y);
                let model = model_without_scale * Affine3A::from_scale(scale);
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
        }
    }
}
