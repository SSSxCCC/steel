use shipyard::{Component, IntoIter, UniqueViewMut, View};
use glam::{Vec4, Vec4Swizzles};
use crate::{render::Canvas, Edit, transform::Transform};

#[derive(Component, Default, Debug)]
pub struct Renderer2D; // can only render cuboid currently. TODO: render multiple shape

impl Edit for Renderer2D {
    fn name() -> &'static str { "Renderer2D" }
}

pub fn renderer2d_to_canvas_system(renderer2d: View<Renderer2D>, transform: View<Transform>, mut canvas: UniqueViewMut<Canvas>) {
    for (transform, renderer2d) in (&transform, &renderer2d).iter() {
        let model = transform.model();
        let vertex = [(model * Vec4::new(-0.5, -0.5, 0.0, 1.0)).xyz(),
            (model * Vec4::new(-0.5, 0.5, 0.0, 1.0)).xyz(),
            (model * Vec4::new(0.5, 0.5, 0.0, 1.0)).xyz(),
            (model * Vec4::new(0.5, -0.5, 0.0, 1.0)).xyz()];
        canvas.rectangles.push(vertex);
    }
}
