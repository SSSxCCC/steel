/// The shader to draw vertices. Used to draw points, lines, and triangles.
pub mod vertex {
    use glam::{Vec3, Vec4};
    use shipyard::EntityId;
    use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex};

    #[derive(BufferContents, Vertex, Clone)]
    #[repr(C)]
    pub struct VertexData {
        #[format(R32G32B32_SFLOAT)]
        pub position: [f32; 3],
        #[format(R32G32B32A32_SFLOAT)]
        pub color: [f32; 4],
        #[format(R32G32_UINT)]
        pub eid: [u32; 2],
    }

    impl VertexData {
        pub fn new(position: Vec3, color: Vec4, eid: EntityId) -> Self {
            VertexData {
                position: position.to_array(),
                color: color.to_array(),
                eid: crate::render::canvas::eid_to_u32_array(eid),
            }
        }
    }

    pub mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: r"
                #version 460

                layout(push_constant) uniform PushConstants {
                    mat4 projection_view;
                } pcs;

                layout(location = 0) in vec3 position;
                layout(location = 1) in vec4 color;
                layout(location = 2) in uvec2 eid;

                layout(location = 0) out vec4 out_color;
                layout(location = 1) out uvec2 out_eid;

                void main() {
                    gl_Position = pcs.projection_view * vec4(position, 1.0);
                    out_color = color;
                    out_eid = eid;
                }
            ",
        }
    }

    pub mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            src: r"
                #version 460

                layout(location = 0) in vec4 in_color;
                layout(location = 1) flat in uvec2 in_eid;

                layout(location = 0) out vec4 f_color;
                layout(location = 1) out uvec2 f_eid;

                void main() {
                    f_color = in_color;
                    f_color = vec4(pow(f_color.xyz, vec3(1.0 / 2.2)), f_color.w); // gamma correction
                    if (f_color.w > 0.0001) {
                        f_eid = in_eid;
                    }
                }
            ",
        }
    }
}

/// Shapes that have static vertices can use this shader to draw.
/// Used to draw rectangles and cuboids.
pub mod shape {
    use glam::{Affine3A, Mat4, Vec3, Vec4};
    use shipyard::EntityId;
    use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex};

    #[derive(BufferContents, Vertex, Clone)]
    #[repr(C)]
    pub struct VertexData {
        #[format(R32G32B32_SFLOAT)]
        pub position: [f32; 3],
    }

    impl VertexData {
        pub fn new(position: Vec3) -> Self {
            VertexData {
                position: position.to_array(),
            }
        }
    }

    #[derive(BufferContents, Vertex, Clone)]
    #[repr(C)]
    pub struct InstanceData {
        #[format(R32G32B32A32_SFLOAT)]
        pub color: [f32; 4],
        #[format(R32G32_UINT)]
        pub eid: [u32; 2],
        #[format(R32G32B32A32_SFLOAT)]
        pub model: [[f32; 4]; 4],
    }

    impl InstanceData {
        pub fn new(color: Vec4, eid: EntityId, model: Affine3A) -> Self {
            InstanceData {
                color: color.to_array(),
                eid: crate::render::canvas::eid_to_u32_array(eid),
                model: Mat4::from(model).to_cols_array_2d(),
            }
        }
    }

    pub mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: r"
                #version 460

                layout(push_constant) uniform PushConstants {
                    mat4 projection_view;
                } pcs;

                layout(location = 0) in vec3 position;
                // instance data
                layout(location = 1) in vec4 color;
                layout(location = 2) in uvec2 eid;
                layout(location = 3) in mat4 model;

                layout(location = 0) out vec4 out_color;
                layout(location = 1) out uvec2 out_eid;

                void main() {
                    gl_Position = pcs.projection_view * model * vec4(position, 1.0);
                    out_color = color;
                    out_eid = eid;
                }
            ",
        }
    }

    pub mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            src: r"
                #version 460

                layout(location = 0) flat in vec4 in_color;
                layout(location = 1) flat in uvec2 in_eid;

                layout(location = 0) out vec4 f_color;
                layout(location = 1) out uvec2 f_eid;

                void main() {
                    f_color = in_color;
                    f_color = vec4(pow(f_color.xyz, vec3(1.0 / 2.2)), f_color.w); // gamma correction
                    if (f_color.w > 0.0001) {
                        f_eid = in_eid;
                    }
                }
            ",
        }
    }
}

/// The shader to draw circles.
/// Use [super::shape::VertexData] and [super::shape::InstanceData].
pub mod circle {
    pub mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: r"
                #version 460

                layout(push_constant) uniform PushConstants {
                    mat4 projection_view;
                } pcs;

                layout(location = 0) in vec3 position;
                // instance data
                layout(location = 1) in vec4 color;
                layout(location = 2) in uvec2 eid;
                layout(location = 3) in mat4 model; // position is center, scale is radius

                layout(location = 0) out vec2 out_position;
                layout(location = 1) out vec4 out_color;
                layout(location = 2) out uvec2 out_eid;

                void main() {
                    gl_Position = pcs.projection_view * model * vec4(position, 1.0);
                    out_position = position.xy;
                    out_color = color;
                    out_eid = eid;
                }
            ",
        }
    }

    pub mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            src: r"
                #version 460

                layout(location = 0) in vec2 in_position;
                layout(location = 1) flat in vec4 in_color;
                layout(location = 2) flat in uvec2 in_eid;

                layout(location = 0) out vec4 f_color;
                layout(location = 1) out uvec2 f_eid;

                void main() {
                    if (distance(vec2(0, 0), in_position) > 0.5) {
                        discard;
                    }
                    f_color = in_color;
                    f_color = vec4(pow(f_color.xyz, vec3(1.0 / 2.2)), f_color.w); // gamma correction
                    if (f_color.w > 0.0001) {
                        f_eid = in_eid;
                    }
                }
            ",
        }
    }
}

/// The shader to draw textures.
/// Use [super::shape::VertexData].
pub mod texture {
    use glam::{Affine3A, Mat4, Vec4};
    use shipyard::EntityId;
    use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex};

    #[derive(BufferContents, Vertex, Clone)]
    #[repr(C)]
    pub struct InstanceData {
        #[format(R32G32B32A32_SFLOAT)]
        pub color: [f32; 4],
        #[format(R32G32_UINT)]
        pub eid: [u32; 2],
        #[format(R32_UINT)]
        pub index: u32,
        #[format(R32G32B32A32_SFLOAT)]
        pub model: [[f32; 4]; 4],
    }

    impl InstanceData {
        pub fn new(color: Vec4, eid: EntityId, index: usize, model: Affine3A) -> Self {
            InstanceData {
                color: color.to_array(),
                eid: crate::render::canvas::eid_to_u32_array(eid),
                index: index as u32,
                model: Mat4::from(model).to_cols_array_2d(),
            }
        }
    }

    pub mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: r"
                #version 460

                layout(push_constant) uniform PushConstants {
                    mat4 projection_view;
                } pcs;

                layout(location = 0) in vec3 position;
                // instance data
                layout(location = 1) in vec4 color;
                layout(location = 2) in uvec2 eid;
                layout(location = 3) in uint index;
                layout(location = 4) in mat4 model;

                layout(location = 0) out vec2 tex_coord;
                layout(location = 1) out vec4 out_color;
                layout(location = 2) out uvec2 out_eid;
                layout(location = 3) out uint out_index;

                void main() {
                    gl_Position = pcs.projection_view * model * vec4(position, 1.0);
                    tex_coord = position.xy * vec2(1.0, -1.0) + vec2(0.5);
                    out_color = color;
                    out_eid = eid;
                    out_index = index;
                }
            ",
        }
    }

    pub mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            src: r"
                #version 460
                #extension GL_EXT_nonuniform_qualifier : require

                layout(set = 0, binding = 0) uniform sampler2D[] tex;

                layout(location = 0) in vec2 tex_coord;
                layout(location = 1) flat in vec4 in_color;
                layout(location = 2) flat in uvec2 in_eid;
                layout(location = 3) flat in uint i;

                layout(location = 0) out vec4 f_color;
                layout(location = 1) out uvec2 f_eid;

                void main() {
                    f_color = in_color * texture(tex[i], tex_coord);
                    if (f_color.w == 0) {
                        discard;
                    }
                    f_color = vec4(pow(f_color.xyz, vec3(1.0 / 2.2)), f_color.w); // gamma correction
                    if (f_color.w > 0.0001) {
                        f_eid = in_eid;
                    }
                }
            ",
        }
    }
}

/// The shader to draw models.
/// Use [super::texture::InstanceData].
pub mod model {
    use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex};

    #[derive(BufferContents, Vertex, Clone)]
    #[repr(C)]
    pub struct VertexData {
        #[format(R32G32B32_SFLOAT)]
        pub position: [f32; 3],
        #[format(R32G32_SFLOAT)]
        pub tex_coord: [f32; 2],
    }

    pub mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: r"
                #version 460

                layout(push_constant) uniform PushConstants {
                    mat4 projection_view;
                } pcs;

                layout(location = 0) in vec3 position;
                layout(location = 1) in vec2 tex_coord;
                // instance data
                layout(location = 2) in vec4 color;
                layout(location = 3) in uvec2 eid;
                layout(location = 4) in uint index;
                layout(location = 5) in mat4 model;

                layout(location = 0) out vec2 out_tex_coord;
                layout(location = 1) out vec4 out_color;
                layout(location = 2) out uvec2 out_eid;
                layout(location = 3) out uint out_index;

                void main() {
                    gl_Position = pcs.projection_view * model * vec4(position, 1.0);
                    out_tex_coord = tex_coord;
                    out_color = color;
                    out_eid = eid;
                    out_index = index;
                }
            ",
        }
    }

    pub mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            src: r"
                #version 460
                #extension GL_EXT_nonuniform_qualifier : require

                layout(set = 0, binding = 0) uniform sampler2D[] tex;

                layout(location = 0) in vec2 tex_coord;
                layout(location = 1) flat in vec4 in_color;
                layout(location = 2) flat in uvec2 in_eid;
                layout(location = 3) flat in uint i;

                layout(location = 0) out vec4 f_color;
                layout(location = 1) out uvec2 f_eid;

                void main() {
                    const uint MAX_UINT = 4294967295u;
                    f_color = in_color;
                    if (i != MAX_UINT) {
                        f_color *= texture(tex[i], tex_coord);
                    }
                    if (f_color.w == 0) {
                        discard;
                    }
                    f_color = vec4(pow(f_color.xyz, vec3(1.0 / 2.2)), f_color.w); // gamma correction
                    if (f_color.w > 0.0001) {
                        f_eid = in_eid;
                    }
                }
            ",
        }
    }
}
