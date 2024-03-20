use std::sync::Arc;
use shipyard::{Component, IntoIter, UniqueViewMut, View};
use glam::{Vec2, Vec4, Vec4Swizzles};
use steel_common::{ComponentData, Limit, Value};
use crate::{render::canvas::Canvas, Edit, transform::Transform};

pub enum ShapeType {
    Circle,
    Rectangle,
}

pub trait Shape : std::fmt::Debug {
    fn shape_type(&self) -> ShapeType;
}

#[derive(Debug)]
pub struct Circle {
    pub radius: f32,
}

impl Circle {
    pub fn new(radius: f32) -> Self {
        Circle { radius }
    }
}

impl Shape for Circle {
    fn shape_type(&self) -> ShapeType {
        ShapeType::Circle
    }
}

#[derive(Debug)]
pub struct Rectangle {
    pub size: Vec2,
}

impl Rectangle {
    pub fn new(size: Vec2) -> Self {
        Rectangle { size }
    }
}

impl Shape for Rectangle {
    fn shape_type(&self) -> ShapeType {
        ShapeType::Rectangle
    }
}

#[derive(Debug)]
pub struct SharedShape(pub Arc<dyn Shape + Send + Sync>);

impl SharedShape {
    pub fn new(shape: impl Shape + Send + Sync + 'static) -> Self {
        SharedShape(Arc::new(shape))
    }
}

impl std::ops::Deref for SharedShape {
    type Target = Arc<dyn Shape + Send + Sync>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Component, Debug)]
pub struct Renderer2D {
    pub shape: SharedShape,
    pub color: Vec4,
}

impl Default for Renderer2D {
    fn default() -> Self {
        Self { shape: SharedShape::new(Rectangle::new(Vec2::ONE)), color: Vec4::ONE /* white */ }
    }
}

impl Edit for Renderer2D {
    fn name() -> &'static str { "Renderer2D" }

    fn get_data(&self) -> ComponentData {
        let mut data = ComponentData::new();
        data.add("shape", Value::Int32(self.shape.shape_type() as i32),
            Limit::Int32Enum(vec![(0, "Circle".into()), (1, "Rectangle".into())]));
        data.add("color", Value::Vec4(self.color), Limit::Vec4Color);
        data
    }

    fn set_data(&mut self, data: &ComponentData) {
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
