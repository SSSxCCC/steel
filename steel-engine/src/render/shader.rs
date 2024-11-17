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
                if (f_color.w > 0.0001) {
                    f_eid = in_eid;
                }
            }
        ",
    }
}

pub mod circle {
    pub mod vs {
        vulkano_shaders::shader! { // TODO: Use different PushConstants in vs and fs
            ty: "vertex",
            src: r"
                #version 460

                layout(push_constant) uniform PushConstants {
                    mat4 projection_view;
                    vec3 center;
                    float radius;
                } pcs;

                layout(location = 0) in vec3 position;
                layout(location = 1) in vec4 color;
                layout(location = 2) in uvec2 eid;

                layout(location = 0) out vec3 out_position;
                layout(location = 1) out vec4 out_color;
                layout(location = 2) out uvec2 out_eid;

                void main() {
                    gl_Position = pcs.projection_view * vec4(position, 1.0);
                    out_position = position;
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

                layout(push_constant) uniform PushConstants {
                    mat4 projection_view;
                    vec3 center;
                    float radius;
                } pcs;

                layout(location = 0) in vec3 in_position;
                layout(location = 1) in vec4 in_color;
                layout(location = 2) flat in uvec2 in_eid;

                layout(location = 0) out vec4 f_color;
                layout(location = 1) out uvec2 f_eid;

                void main() {
                    if (distance(pcs.center, in_position) > pcs.radius) {
                        discard;
                    }
                    f_color = in_color;
                    if (f_color.w > 0.0001) {
                        f_eid = in_eid;
                    }
                }
            ",
        }
    }
}

pub mod texture {
    pub mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: r"
                #version 460

                layout(push_constant) uniform PushConstants {
                    mat4 projection_view_model; // TODO: instanced rendering
                } pcs;

                layout(location = 0) in vec3 position;
                layout(location = 1) in vec4 color;
                layout(location = 2) in uvec2 eid;

                layout(location = 0) out vec2 tex_coord;
                layout(location = 1) out vec4 out_color;
                layout(location = 2) out uvec2 out_eid;

                void main() {
                    gl_Position = pcs.projection_view_model * vec4(position, 1.0);
                    tex_coord = position.xy * vec2(1.0, -1.0) + vec2(0.5);
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

                layout(set = 0, binding = 0) uniform sampler s;
                layout(set = 0, binding = 1) uniform texture2D tex;

                layout(location = 0) in vec2 tex_coord;
                layout(location = 1) in vec4 in_color;
                layout(location = 2) flat in uvec2 in_eid;

                layout(location = 0) out vec4 f_color;
                layout(location = 1) out uvec2 f_eid;

                void main() {
                    f_color = in_color * texture(sampler2D(tex, s), tex_coord);
                    if (f_color.w > 0.0001) {
                        f_eid = in_eid;
                    }
                }
            ",
        }
    }
}
