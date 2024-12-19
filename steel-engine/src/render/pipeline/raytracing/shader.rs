pub mod raygen {
    vulkano_shaders::shader! {
        ty: "raygen",
        spirv_version: "1.4",
        src: r"
            #version 460
            #extension GL_EXT_ray_tracing : require

            // =============== Random ===============

            struct PCG32si {
                uint state;
            };

            // Step function for PCG32
            void pcg_oneseq_32_step_r(inout PCG32si rng) {
                const uint PCG_DEFAULT_MULTIPLIER_32 = 747796405u;
                const uint PCG_DEFAULT_INCREMENT_32 = 2891336453u;
                rng.state = (rng.state * PCG_DEFAULT_MULTIPLIER_32 + PCG_DEFAULT_INCREMENT_32);
            }

            // PCG output function
            uint pcg_output_rxs_m_xs_32_32(uint state) {
                uint word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
                return (word >> 22u) ^ word;
            }

            // Create a new RNG with a seed
            PCG32si pcg_new(uint seed) {
                PCG32si rng;
                rng.state = seed;
                pcg_oneseq_32_step_r(rng);
                rng.state += seed;  // equivalent to wrapping_add
                pcg_oneseq_32_step_r(rng);
                return rng;
            }

            // Generate a random uint
            uint next_u32(inout PCG32si rng) {
                uint old_state = rng.state;
                pcg_oneseq_32_step_r(rng);
                return pcg_output_rxs_m_xs_32_32(old_state);
            }

            // Generate a random float [0.0, 1.0)
            float next_f32(inout PCG32si rng) {
                const uint float_size = 32u;  // Number of bits in a float
                const uint float_precision = 24u;  // Precision for floating point numbers (23 bits + 1 sign bit)
                const float scale = 1.0 / float(1 << float_precision);

                uint value = next_u32(rng);
                value >>= (float_size - float_precision);  // Shift to get the desired precision
                return scale * float(value);
            }

            // Generate a random float in the range [min, max]
            float next_f32_range(inout PCG32si rng, float min, float max) {
                return min + (max - min) * next_f32(rng);
            }

            // =============== Math ===============

            #define PI 3.1415926538

            vec3 random_in_unit_sphere(inout PCG32si rng) {
                // Generate random spherical coordinates (direction)
                float theta = next_f32_range(rng, 0.0, 2.0 * PI);  // Uniform azimuthal angle
                float phi = next_f32_range(rng, -1.0, 1.0);        // Uniform cosine of polar angle

                // Sample radius as the cube root of a uniform random value to ensure uniform distribution in volume
                float r = pow(next_f32(rng), 1.0 / 3.0);  // Cube root of a uniform random number in [0, 1]

                // Convert spherical coordinates (r, theta, phi) to Cartesian coordinates
                float x = r * sqrt(1.0 - phi * phi) * cos(theta);
                float y = r * sqrt(1.0 - phi * phi) * sin(theta);
                float z = r * phi;

                return vec3(x, y, z);
            }

            vec3 random_in_hemisphere(vec3 normal, inout PCG32si rng) {
                vec3 v = normalize(random_in_unit_sphere(rng));
                if (dot(normal, v) > 0.0) {
                    return v;
                } else {
                    return -v;
                }
            }

            vec3 random_in_unit_disk(inout PCG32si rng) {
                // Generate random angle between 0 and 2π
                float theta = next_f32_range(rng, 0.0, 2.0 * PI);

                // Generate random radius squared between 0 and 1, then take the square root to make it uniform in area
                float r2 = next_f32(rng);  // Uniformly sample r^2 (radius squared)
                float r = sqrt(r2);        // Take square root to get radius

                // Convert polar coordinates to Cartesian coordinates
                float x = r * cos(theta);
                float y = r * sin(theta);

                return vec3(x, y, 0.0);
            }

            // =============== Ray and Payload structs ===============

            struct Ray {
                vec3 origin;
                vec3 direction;
            };

            Ray default_Ray() {
                return Ray(vec3(0.0), vec3(0.0));
            }

            struct RayPayload {
                vec3 position;
                vec3 normal;
                bool is_miss;
                uint material;
                bool front_face;
            };

            RayPayload default_RayPayload() {
                RayPayload payload;
                payload.position = vec3(0.0);
                payload.normal = vec3(0.0);
                payload.is_miss = false;
                payload.material = 0u;
                payload.front_face = false;
                return payload;
            }

            // =============== Materials ===============

            float reflectance(float cosine, float ref_idx) {
                float r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
                r0 = r0 * r0;
                return r0 + (1.0 - r0) * pow(1.0 - cosine, 5.0);
            }

            struct Scatter {
                vec3 color;
                Ray ray;
            };

            Scatter default_Scatter() {
                return Scatter(vec3(0.0), default_Ray());
            }

            // Materials
            struct Lambertian {
                vec3 albedo;
            };

            struct Metal {
                vec3 albedo;
                float fuzz;
            };

            struct Dielectric {
                float ir;
            };

            // Scatter functions for different materials
            bool scatter_Lambertian(Lambertian material, Ray ray, RayPayload ray_payload, inout PCG32si rng, inout Scatter scatter) {
                vec3 scatter_direction = ray_payload.normal + normalize(random_in_unit_sphere(rng));
                scatter_direction = (length(scatter_direction) < 1e-8) ? ray_payload.normal : scatter_direction;

                scatter.ray.origin = ray_payload.position;
                scatter.ray.direction = scatter_direction;
                scatter.color = material.albedo;

                return true;
            }

            bool scatter_Metal(Metal material, Ray ray, RayPayload ray_payload, inout PCG32si rng, inout Scatter scatter) {
                vec3 reflected = reflect(normalize(ray.direction), ray_payload.normal);
                vec3 scatter_direction = reflected + material.fuzz * random_in_unit_sphere(rng);

                if (dot(scatter_direction, ray_payload.normal) > 0.0) {
                    scatter.ray.origin = ray_payload.position;
                    scatter.ray.direction = scatter_direction;
                    scatter.color = material.albedo;
                    return true;
                }
                return false;
            }

            bool scatter_Dielectric(Dielectric material, Ray ray, RayPayload ray_payload, inout PCG32si rng, inout Scatter scatter) {
                float refraction_ratio = ray_payload.front_face ? (1.0 / material.ir) : material.ir;
                vec3 unit_direction = normalize(ray.direction);
                float cos_theta = min(dot(-unit_direction, ray_payload.normal), 1.0);
                float sin_theta = sqrt(1.0 - cos_theta * cos_theta);
                bool cannot_refract = refraction_ratio * sin_theta > 1.0;

                vec3 direction = (cannot_refract || reflectance(cos_theta, refraction_ratio) > next_f32(rng))
                    ? reflect(unit_direction, ray_payload.normal)
                    : refract(unit_direction, ray_payload.normal, refraction_ratio);

                scatter.ray.origin = ray_payload.position;
                scatter.ray.direction = direction;
                scatter.color = vec3(1.0, 1.0, 1.0); // White for Dielectric

                return true;
            }

            // Scatter function for EnumMaterial
            struct EnumMaterial {
                vec4 data;
                uint t;
            };

            bool scatter_EnumMaterial(EnumMaterial material, Ray ray, RayPayload ray_payload, inout PCG32si rng, inout Scatter scatter) {
                if (material.t == 0u) {
                    Lambertian material = Lambertian(material.data.xyz);
                    return scatter_Lambertian(material, ray, ray_payload, rng, scatter);
                } else if (material.t == 1u) {
                    Metal material = Metal(material.data.xyz, material.data.w);
                    return scatter_Metal(material, ray, ray_payload, rng, scatter);
                } else if (material.t == 2u) {
                    Dielectric material = Dielectric(material.data.x);
                    return scatter_Dielectric(material, ray, ray_payload, rng, scatter);
                } else {
                    return false;
                }
            }

            // =============== Camera ===============

            // Camera structure
            struct Camera {
                vec3 origin;
                uint type; // 0 is orthographic, 1 is perspective
                vec3 lower_left_corner;
                vec3 horizontal;
                vec3 vertical;
                float focus_dist;
                vec3 u; // right
                vec3 v; // up
                vec3 w; // forward
                float lens_radius;
            };

            // Camera creation function
            Camera create_camera(uint type, vec3 origin, vec3 direction, float data, float aspect_ratio, float lens_radius, float focus_dist) {
                vec3 w = normalize(direction);
                vec3 u = normalize(cross(w, vec3(0.0, 1.0, 0.0)));
                vec3 v = cross(u, w);

                vec3 horizontal;
                vec3 vertical;
                if (type == 0) { // orthographic
                    float viewport_height = data;
                    float viewport_width = aspect_ratio * viewport_height;
                    horizontal = viewport_width * u;
                    vertical = viewport_height * v;
                } else { // perspective
                    float vfov = data;
                    float viewport_height = 2.0 * tan(vfov / 2.0);
                    float viewport_width = aspect_ratio * viewport_height;
                    horizontal = focus_dist * viewport_width * u;
                    vertical = focus_dist * viewport_height * v;
                }
                vec3 lower_left_corner = origin - horizontal / 2.0 - vertical / 2.0 + focus_dist * w;

                Camera cam;
                cam.type = type;
                cam.origin = origin;
                cam.lower_left_corner = lower_left_corner;
                cam.horizontal = horizontal;
                cam.vertical = vertical;
                cam.focus_dist = focus_dist;
                cam.u = u;
                cam.v = v;
                cam.w = w;
                cam.lens_radius = lens_radius;
                return cam;
            }

            // Function to generate a ray from the camera
            Ray get_ray(Camera cam, float s, float t, inout PCG32si rng) {
                vec3 rd = cam.lens_radius * random_in_unit_disk(rng);
                vec3 offset = cam.u * rd.x + cam.v * rd.y;
                vec3 look_at = cam.lower_left_corner + s * cam.horizontal + t * cam.vertical;
                Ray r;
                if (cam.type == 0) { // orthographic
                    r.origin = look_at - cam.focus_dist * cam.w + offset;
                } else { // perspective
                    r.origin = cam.origin + offset;
                }
                r.direction = normalize(look_at - r.origin);
                return r;
            }

            // =============== Shader ===============

            layout(set = 0, binding = 0) uniform accelerationStructureEXT top_level_as;
            layout(set = 0, binding = 1, rgba8) uniform image2D out_image;
            layout(set = 0, binding = 2) buffer Materials {
                EnumMaterial materials[];
            };

            layout(location = 0) rayPayloadEXT RayPayload payload;

            layout(push_constant) uniform PushConstants {
                vec3 camera_position;
                uint camera_type; // 0 is orthographic, 1 is perspective
                vec3 camera_direction;
                float camera_data; // height of orthographic or vfov of perspective
                float camera_lens_radius;
                float camera_focus_dist;
                uint samples;
                uint max_bounces;
                vec3 miss_color_top; // miss color is linear gradient from top to bottom
                uint seed;
                vec3 miss_color_bottom;
            } pcs;

            void main() {
                // Launch ID and size (inbuilt variables in GLSL)
                uvec3 launch_id = gl_LaunchIDEXT;
                uvec3 launch_size = gl_LaunchSizeEXT;

                // Random seed initialization
                uint rand_seed = (launch_id.y * launch_size.x + launch_id.x) ^ pcs.seed;
                PCG32si rng = pcg_new(rand_seed);

                // Camera setup
                Camera camera = create_camera(
                    pcs.camera_type,
                    pcs.camera_position,
                    pcs.camera_direction,
                    pcs.camera_data,
                    float(launch_size.x) / float(launch_size.y),
                    pcs.camera_lens_radius,
                    pcs.camera_focus_dist
                );

                uint cull_mask = 0xff;
                float tmin = 0.001;
                float tmax = 100000.0;

                vec3 final_color = vec3(0.0);

                for (uint i = 0; i < pcs.samples; i++) {
                    float u = (float(launch_id.x) + next_f32(rng)) / float(launch_size.x);
                    float v = (float(launch_id.y) + next_f32(rng)) / float(launch_size.y);

                    vec3 color = vec3(1.0);
                    Ray ray = get_ray(camera, u, v, rng);

                    for (uint j = 0; j <= pcs.max_bounces; j++) {
                        payload = default_RayPayload();
                        traceRayEXT(
                            top_level_as,
                            gl_RayFlagsOpaqueEXT,
                            cull_mask,
                            0, 0, 0,
                            ray.origin, tmin, ray.direction, tmax,
                            0
                        );

                        if (payload.is_miss) {
                            color *= payload.position;
                            break;
                        } else {
                            Scatter scatter = default_Scatter();
                            if (scatter_EnumMaterial(materials[payload.material], ray, payload, rng, scatter)) {
                                color *= scatter.color;
                                ray = scatter.ray;
                            } else {
                                break;
                            }
                        }
                    }

                    final_color += color;
                }

                final_color = final_color / float(pcs.samples);
                final_color = pow(final_color, vec3(1.0 / 2.2)); // gamma correction

                ivec2 pos = ivec2(launch_id.xy);
                pos.y = int(launch_size.y) - 1 - pos.y;

                imageStore(out_image, pos, vec4(final_color, 1.0));
            }
        ",
    }
}

pub mod miss {
    vulkano_shaders::shader! {
        ty: "miss",
        spirv_version: "1.4",
        src: r"
            #version 460
            #extension GL_EXT_ray_tracing : require

            layout(push_constant) uniform PushConstants {
                vec3 camera_position;
                uint camera_type; // 0 is orthographic, 1 is perspective
                vec3 camera_direction;
                float camera_data; // height of orthographic or vfov of perspective
                float camera_lens_radius;
                float camera_focus_dist;
                uint samples;
                uint max_bounces;
                vec3 miss_color_top; // miss color is linear gradient between top and bottom
                uint seed;
                vec3 miss_color_bottom;
            } pcs;

            layout(location = 0) rayPayloadInEXT RayPayload {
                vec3 position;
                vec3 normal;
                bool is_miss;
                uint material;
                bool front_face;
            } payload;

            void main() {
                vec3 world_ray_direction = normalize(gl_WorldRayDirectionEXT);
                float t = 0.5 * (world_ray_direction.y + 1.0);
                vec3 color = mix(pcs.miss_color_bottom, pcs.miss_color_top, t);

                payload.is_miss = true;
                payload.position = color;
                payload.normal = vec3(0.0, 0.0, 0.0);
                payload.material = 0;
                payload.front_face = false;
            }
        ",
    }
}

pub mod sphere_intersection {
    vulkano_shaders::shader! {
        ty: "intersection",
        spirv_version: "1.4",
        src: r"
            #version 460
            #extension GL_EXT_ray_tracing : require

            hitAttributeEXT float t;

            void main() {
                vec3 ray_origin = gl_ObjectRayOriginEXT;
                vec3 ray_direction = gl_ObjectRayDirectionEXT;
                float t_min = gl_RayTminEXT;
                float t_max = gl_RayTmaxEXT;

                vec3 oc = ray_origin;
                float a = dot(ray_direction, ray_direction);
                float half_b = dot(oc, ray_direction);
                float c = dot(oc, oc) - 0.5 * 0.5;

                float discriminant = half_b * half_b - a * c;
                if (discriminant < 0.0) {
                    return;  // No intersection
                }

                float sqrtd = sqrt(discriminant);
                float root0 = (-half_b - sqrtd) / a;
                float root1 = (-half_b + sqrtd) / a;

                if (root0 >= t_min && root0 <= t_max) {
                    t = root0;
                    reportIntersectionEXT(root0, 0);  // Report intersection
                }

                if (root1 >= t_min && root1 <= t_max) {
                    t = root1;
                    reportIntersectionEXT(root1, 0);  // Report intersection
                }
            }
        ",
    }
}

pub mod sphere_closesthit {
    vulkano_shaders::shader! {
        ty: "closesthit",
        spirv_version: "1.4",
        src: r"
            #version 460
            #extension GL_EXT_ray_tracing : require

            struct RayPayload {
                vec3 position;
                vec3 normal;
                bool is_miss;
                uint material;
                bool front_face;
            };

            RayPayload new_RayPayload(vec3 position, vec3 outward_normal, vec3 ray_direction, uint material) {
                bool front_face = dot(ray_direction, outward_normal) < 0.0;
                vec3 normal = front_face ? outward_normal : -outward_normal;

                return RayPayload(
                    position,
                    normal,
                    false, // is_miss initialized to false
                    material,
                    front_face
                );
            }

            hitAttributeEXT float t;
            layout(location = 0) rayPayloadInEXT RayPayload payload;

            void main() {
                vec3 hit_pos = gl_WorldRayOriginEXT + t * gl_WorldRayDirectionEXT;
                vec3 normal = normalize(hit_pos - gl_ObjectToWorldEXT[3]);
                payload = new_RayPayload(hit_pos, normal, gl_WorldRayDirectionEXT, gl_InstanceCustomIndexEXT);
            }
        ",
    }
}
