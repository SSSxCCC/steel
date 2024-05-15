use parry2d::shape::{ShapeType, SharedShape};
use shipyard::{Component, IntoIter, IntoWithId, UniqueViewMut, View};
use glam::{Vec4, Vec4Swizzles};
use steel_common::data::{Data, Limit, Value};
use crate::{edit::Edit, render::canvas::Canvas, shape::ShapeWrapper, transform::Transform};

/// Renderer2D component is used to draw a 2D shape on an entity.
#[derive(Component, Debug)]
pub struct Renderer2D {
    pub shape: ShapeWrapper,
    pub color: Vec4,
}

impl Default for Renderer2D {
    fn default() -> Self {
        Self { shape: ShapeWrapper(SharedShape::cuboid(0.5, 0.5)), color: Vec4::ONE /* white */ }
    }
}

impl Edit for Renderer2D {
    fn name() -> &'static str { "Renderer2D" }

    fn get_data(&self) -> Data {
        let mut data = Data::new();
        self.shape.get_data(&mut data);
        data.insert_with_limit("color", Value::Vec4(self.color), Limit::Vec4Color)
    }

    fn set_data(&mut self, data: &Data) {
        self.shape.set_data(data);
        if let Some(Value::Vec4(v)) = data.get("color") { self.color = *v }
    }
}

/// Add drawing data to the Canvas unique according to the Renderer2D components.
pub fn renderer2d_to_canvas_system(renderer2d: View<Renderer2D>, transform: View<Transform>, mut canvas: UniqueViewMut<Canvas>) {
    for (eid, (transform, renderer2d)) in (&transform, &renderer2d).iter().with_id() {
        match renderer2d.shape.shape_type() {
            ShapeType::Ball => {
                let scale = std::cmp::max_by(transform.scale.x.abs(), transform.scale.y.abs(), |x, y| x.partial_cmp(y).unwrap());
                let radius = renderer2d.shape.as_ball().unwrap().radius * scale;
                canvas.circle(transform.position, transform.rotation, radius, renderer2d.color, eid);
            },
            ShapeType::Cuboid => {
                let shape = renderer2d.shape.as_cuboid().unwrap();
                let (half_width, half_height) = (shape.half_extents.x, shape.half_extents.y);
                let model = transform.model();
                canvas.rectangle((model * Vec4::new(-half_width, -half_height, 0.0, 1.0)).xyz(),
                    (model * Vec4::new(-half_width, half_height, 0.0, 1.0)).xyz(),
                    (model * Vec4::new(half_width, half_height, 0.0, 1.0)).xyz(),
                    (model * Vec4::new(half_width, -half_height, 0.0, 1.0)).xyz(),
                    renderer2d.color, eid);
            },
            _ => (),
        }
    }
}
