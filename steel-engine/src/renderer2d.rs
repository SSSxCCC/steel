use shipyard::{Component, IntoIter, UniqueViewMut, View};
use glam::{Mat4, Quat, Vec3, Vec4, Vec4Swizzles};
use crate::{render::Canvas, Edit, transform::Transform2D};

#[derive(Component, Default, Debug)]
pub struct Renderer2D; // can only render cuboid currently. TODO: render multiple shape

impl Edit for Renderer2D {
    fn name() -> &'static str { "Renderer2D" }
}

pub fn renderer2d_to_canvas_system(renderer2d: View<Renderer2D>, transform2d: View<Transform2D>, mut canvas: UniqueViewMut<Canvas>) {
    for (transform2d, renderer2d) in (&transform2d, &renderer2d).iter() {
        let model = Mat4::from_scale_rotation_translation(Vec3 { x: transform2d.scale.x, y: transform2d.scale.y, z: 1.0 },
            Quat::from_axis_angle(Vec3::Z, transform2d.rotation), transform2d.position);
        let vertex = [(model * Vec4::new(-0.5, -0.5, 0.0, 1.0)).xyz(),
            (model * Vec4::new(-0.5, 0.5, 0.0, 1.0)).xyz(),
            (model * Vec4::new(0.5, 0.5, 0.0, 1.0)).xyz(),
            (model * Vec4::new(0.5, -0.5, 0.0, 1.0)).xyz()];
        canvas.rectangles.push(vertex);
    }
}
