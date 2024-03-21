use glam::Vec2;
use indexmap::IndexMap;
use parry2d::shape::{Ball, Cuboid, ShapeType, SharedShape};
use steel_common::{ComponentData, Limit, Value};

pub struct ShapeWrapper(pub SharedShape);

impl std::ops::Deref for ShapeWrapper {
    type Target = SharedShape;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Debug for ShapeWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ShapeWrapper").field(&self.shape_type()).finish() // TODO: print all members
    }
}

impl ShapeWrapper {
    fn i32_to_shape_type(i: &i32) -> ShapeType {
        match i {
            0 => ShapeType::Ball,
            1 => ShapeType::Cuboid,
            _ => ShapeType::Ball, // TODO: support all shape type
        }
    }

    pub fn get_data(&self, data: &mut ComponentData) {
        data.add("shape_type", Value::Int32(self.shape_type() as i32),
            Limit::Int32Enum(vec![(0, "Ball".into()), (1, "Cuboid".into())]));
        if let Some(shape) = self.as_ball() {
            data.values.insert("radius".into(), Value::Float32(shape.radius));
        } else if let Some(shape) = self.as_cuboid() {
            data.values.insert("size".into(), Value::Vec2(Vec2::new(shape.half_extents.x * 2.0, shape.half_extents.y * 2.0)));
        } // TODO: support all shape type
    }

    pub fn set_data(&mut self, data: &ComponentData) {
        if let Some(Value::Int32(i)) = data.values.get("shape_type") {
            let shape_type = Self::i32_to_shape_type(i);
            match shape_type {
                ShapeType::Ball => { // We have to create a new shape because SharedShape::as_shape_mut method can not compile
                    let mut shape = if let Some(shape) = self.as_ball() { *shape } else { Ball::new(0.5) };
                    if let Some(Value::Float32(f)) = data.values.get("radius") { shape.radius = *f }
                    self.0 = SharedShape::new(shape);
                },
                ShapeType::Cuboid => {
                    let mut shape = if let Some(shape) = self.as_cuboid() { *shape } else { Cuboid::new([0.5, 0.5].into()) };
                    if let Some(Value::Vec2(v)) = data.values.get("size") { (shape.half_extents.x, shape.half_extents.y) = (v.x / 2.0, v.y / 2.0) }
                    self.0 = SharedShape::new(shape);
                },
                _ => (), // TODO: support all shape type
            }
        }
    }
}
