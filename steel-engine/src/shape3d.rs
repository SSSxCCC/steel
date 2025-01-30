use glam::Vec3;
use parry3d::shape::{Ball, Cuboid, ShapeType, SharedShape};
use steel_common::data::{Data, Limit, Value};

/// A wrapper of [parry3d::shape::SharedShape].
pub struct Shape3D(pub SharedShape);

impl std::ops::Deref for Shape3D {
    type Target = SharedShape;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Shape3D {
    fn default() -> Self {
        Self(SharedShape::cuboid(0.5, 0.5, 0.5))
    }
}

impl std::fmt::Debug for Shape3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Shape").field(&self.shape_type()).finish() // TODO: print all members
    }
}

impl Shape3D {
    /// Convert i32 to ShapeType.
    fn i32_to_shape_type(i: &i32) -> ShapeType {
        match i {
            0 => ShapeType::Ball,
            1 => ShapeType::Cuboid,
            _ => ShapeType::Ball, // TODO: support all shape type
        }
    }

    /// Hepler funtion to edit the shape, used in Edit::get_data.
    pub fn get_data(&self, data: &mut Data) {
        data.add_value_with_limit(
            "shape_type",
            Value::Int32(self.shape_type() as i32),
            Limit::Int32Enum(vec![(0, "Sphere".into()), (1, "Cuboid".into())]),
        );
        if let Some(shape) = self.as_ball() {
            data.values
                .insert("radius".into(), Value::Float32(shape.radius));
        } else if let Some(shape) = self.as_cuboid() {
            data.values.insert(
                "size".into(),
                Value::Vec3(Vec3::new(
                    shape.half_extents.x * 2.0,
                    shape.half_extents.y * 2.0,
                    shape.half_extents.z * 2.0,
                )),
            );
        } // TODO: support all shape type
    }

    /// Hepler funtion to edit the shape, used in Edit::set_data.
    pub fn set_data(&mut self, data: &Data) {
        if let Some(Value::Int32(i)) = data.get("shape_type") {
            let shape_type = Self::i32_to_shape_type(i);
            match shape_type {
                ShapeType::Ball => {
                    // We have to create a new shape because SharedShape::as_shape_mut method can not compile
                    let mut shape = if let Some(shape) = self.as_ball() {
                        *shape
                    } else {
                        Ball::new(0.5)
                    };
                    if let Some(Value::Float32(f)) = data.get("radius") {
                        shape.radius = *f
                    }
                    self.0 = SharedShape::new(shape);
                }
                ShapeType::Cuboid => {
                    let mut shape = if let Some(shape) = self.as_cuboid() {
                        *shape
                    } else {
                        Cuboid::new([0.5, 0.5, 0.5].into())
                    };
                    if let Some(Value::Vec3(v)) = data.get("size") {
                        (
                            shape.half_extents.x,
                            shape.half_extents.y,
                            shape.half_extents.z,
                        ) = (v.x / 2.0, v.y / 2.0, v.z / 2.0)
                    }
                    self.0 = SharedShape::new(shape);
                }
                _ => (), // TODO: support all shape type
            }
        }
    }
}
