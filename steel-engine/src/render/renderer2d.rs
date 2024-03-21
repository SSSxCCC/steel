use parry2d::shape::SharedShape;
use shipyard::{Component, IntoIter, UniqueViewMut, View};
use glam::{Vec4, Vec4Swizzles};
use steel_common::{ComponentData, Limit, Value};
use crate::{render::canvas::Canvas, shape::ShapeWrapper, transform::Transform, Edit};

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

    fn get_data(&self) -> ComponentData {
        let mut data = ComponentData::new();
        self.shape.get_data(&mut data);
        data.add("color", Value::Vec4(self.color), Limit::Vec4Color);
        data
    }

    fn set_data(&mut self, data: &ComponentData) {
        self.shape.set_data(data);
        if let Some(Value::Vec4(v)) = data.values.get("color") { self.color = *v }
    }
}

pub fn renderer2d_to_canvas_system(renderer2d: View<Renderer2D>, transform: View<Transform>, mut canvas: UniqueViewMut<Canvas>) {
    for (transform, renderer2d) in (&transform, &renderer2d).iter() {
        let model = transform.model();
        let vertex = [((model * Vec4::new(-0.5, -0.5, 0.0, 1.0)).xyz(), renderer2d.color),
            ((model * Vec4::new(-0.5, 0.5, 0.0, 1.0)).xyz(), renderer2d.color),
            ((model * Vec4::new(0.5, 0.5, 0.0, 1.0)).xyz(), renderer2d.color),
            ((model * Vec4::new(0.5, -0.5, 0.0, 1.0)).xyz(), renderer2d.color)];
        canvas.rectangles.push(vertex);
    }
}
