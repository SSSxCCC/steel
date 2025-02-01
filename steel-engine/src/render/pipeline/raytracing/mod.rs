pub mod material;

pub(crate) mod util;

mod shader;

use crate::{
    camera::CameraInfo,
    render::{
        canvas::Canvas, mesh::MeshData, texture::TextureData, FrameRenderInfo, RenderContext,
    },
};
use ash::vk;
use glam::{Affine3A, Vec3, Vec4};
use indexmap::IndexSet;
use material::{EnumMaterial, Material};
use rand::{rngs::StdRng, RngCore, SeedableRng};
use shipyard::EntityId;
use std::{collections::HashMap, sync::Arc};
use steel_common::{
    camera::CameraSettings,
    data::{Data, Limit, Value},
};
use util::ash::{AshBuffer, AshPipeline, SbtRegion, ShaderGroup};
use vulkano::{
    acceleration_structure::{AabbPositions, AccelerationStructure},
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer},
    descriptor_set::{
        layout::{DescriptorBindingFlags, DescriptorSetLayout},
        PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::Device,
    image::view::ImageView,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{layout::PipelineDescriptorSetLayoutCreateInfo, PipelineLayout},
    sync::GpuFuture,
    VulkanObject,
};

/// Ray tracing render pipeline settings.
pub struct RayTracingSettings {
    /// The radius of camera lens for simulating depth of field.
    pub camera_lens_radius: f32,
    /// The camera focus distance, where objects appear sharp in depth of field calculations.
    pub camera_focus_dist: f32,
    /// Number of rays we trace per pixel for anti-aliasing.
    pub samples: u32,
    /// Max number of bounces the ray can make in the scene.
    pub max_bounces: u32,
    /// The miss color when ray direction is +Y, miss color is linear gradient between top and bottom.
    pub miss_color_top: Vec3,
    /// The miss color when ray direction is -Y, miss color is linear gradient between top and bottom.
    pub miss_color_bottom: Vec3,
}

impl Default for RayTracingSettings {
    fn default() -> Self {
        RayTracingSettings {
            camera_lens_radius: 0.0,
            camera_focus_dist: 10.0,
            samples: 30,
            max_bounces: 30,
            miss_color_top: Vec3::ONE,
            miss_color_bottom: Vec3::ZERO,
        }
    }
}

impl RayTracingSettings {
    pub fn get_data(&self, data: &mut Data) {
        data.add_value_with_limit(
            "camera_lens_radius",
            Value::Float32(self.camera_lens_radius),
            Limit::Float32Range(0.0..=f32::MAX),
        );
        data.add_value_with_limit(
            "camera_focus_dist",
            Value::Float32(self.camera_focus_dist),
            Limit::Float32Range(0.0..=f32::MAX),
        );
        data.add_value_with_limit(
            "samples",
            Value::UInt32(self.samples),
            Limit::UInt32Range(1..=u32::MAX),
        );
        data.add_value("max_bounces", Value::UInt32(self.max_bounces));
        data.add_value_with_limit(
            "miss_color_top",
            Value::Vec3(self.miss_color_top),
            Limit::Vec3Color,
        );
        data.add_value_with_limit(
            "miss_color_bottom",
            Value::Vec3(self.miss_color_bottom),
            Limit::Vec3Color,
        );
    }

    pub fn set_data(&mut self, data: &Data) {
        if let Some(Value::Float32(v)) = data.get("camera_lens_radius") {
            self.camera_lens_radius = *v;
        }
        if let Some(Value::Float32(v)) = data.get("camera_focus_dist") {
            self.camera_focus_dist = *v;
        }
        if let Some(Value::UInt32(v)) = data.get("samples") {
            self.samples = *v;
        }
        if let Some(Value::UInt32(v)) = data.get("max_bounces") {
            self.max_bounces = *v;
        }
        if let Some(Value::Vec3(v)) = data.get("miss_color_top") {
            self.miss_color_top = *v;
        }
        if let Some(Value::Vec3(v)) = data.get("miss_color_bottom") {
            self.miss_color_bottom = *v;
        }
    }
}

/// RayTracingPipeline stores many render objects that exist between frames.
pub(crate) struct RayTracingPipeline {
    pipeline: AshPipeline,
    pipeline_layout: Arc<PipelineLayout>,
    descriptor_set_layout: Arc<DescriptorSetLayout>,
    #[allow(unused)]
    sbt_buffer: AshBuffer,
    sbt_region: SbtRegion,
    rng: StdRng,
    #[allow(unused)]
    device: Arc<Device>, // device must be destroyed after vk buffer
}

impl RayTracingPipeline {
    pub fn new(context: &RenderContext) -> Self {
        let raygen_shader_module = shader::raygen::load(context.device.clone()).unwrap();
        let miss_shader_module = shader::miss::load(context.device.clone()).unwrap();
        let closesthit_shader_module = shader::closesthit::load(context.device.clone()).unwrap();
        let circle_intersection_shader_module =
            shader::circle_intersection::load(context.device.clone()).unwrap();
        let circle_closesthit_shader_module =
            shader::circle_closesthit::load(context.device.clone()).unwrap();
        let sphere_intersection_shader_module =
            shader::sphere_intersection::load(context.device.clone()).unwrap();
        let sphere_closesthit_shader_module =
            shader::sphere_closesthit::load(context.device.clone()).unwrap();

        let (shader_stages, stages) = util::create_shader_stages([
            raygen_shader_module,
            miss_shader_module,
            closesthit_shader_module,
            circle_intersection_shader_module,
            circle_closesthit_shader_module,
            sphere_intersection_shader_module,
            sphere_closesthit_shader_module,
        ]);

        let shader_groups = util::ash::create_shader_groups([
            ShaderGroup::General(0),
            ShaderGroup::General(1),
            ShaderGroup::TrianglesHitGroup {
                closest_hit_shader: 2,
                any_hit_shader: vk::SHADER_UNUSED_KHR,
            },
            ShaderGroup::ProceduralHitGroup {
                closest_hit_shader: 4,
                any_hit_shader: vk::SHADER_UNUSED_KHR,
                intersection_shader: 3,
            },
            ShaderGroup::ProceduralHitGroup {
                closest_hit_shader: 6,
                any_hit_shader: vk::SHADER_UNUSED_KHR,
                intersection_shader: 5,
            },
        ]);

        let properties = context.device.physical_device().properties();
        let max_descriptor_count = properties
            .max_per_stage_descriptor_samplers
            .min(properties.max_per_stage_descriptor_sampled_images);
        let mut pipeline_descriptor_set_layout_create_info =
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages);
        let bindings = &mut pipeline_descriptor_set_layout_create_info.set_layouts[0].bindings;
        let binding = bindings.get_mut(&(bindings.len() as u32 - 1)).unwrap();
        binding.binding_flags |= DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT;
        binding.descriptor_count = max_descriptor_count;

        let pipeline_layout = PipelineLayout::new(
            context.device.clone(),
            pipeline_descriptor_set_layout_create_info
                .into_pipeline_layout_create_info(context.device.clone())
                .unwrap(),
        )
        .unwrap();
        let descriptor_set_layout = pipeline_layout.set_layouts()[0].clone();

        let pipeline = AshPipeline::new(
            unsafe {
                context.ash.rt_pipeline().create_ray_tracing_pipelines(
                    vk::DeferredOperationKHR::null(),
                    vk::PipelineCache::null(),
                    &[vk::RayTracingPipelineCreateInfoKHR::builder()
                        .stages(&shader_stages)
                        .groups(&shader_groups)
                        .max_pipeline_ray_recursion_depth(1)
                        .layout(pipeline_layout.handle())
                        .build()],
                    None,
                )
            }
            .unwrap()[0],
            context.ash.device().clone(),
        );

        let (sbt_buffer, sbt_region) =
            util::ash::create_sbt_buffer_and_region(&context.ash, *pipeline, shader_groups.len());

        RayTracingPipeline {
            pipeline,
            pipeline_layout,
            descriptor_set_layout,
            sbt_buffer,
            sbt_region,
            rng: StdRng::from_entropy(),
            device: context.device.clone(),
        }
    }

    pub fn draw(
        &mut self,
        context: &RenderContext,
        info: &FrameRenderInfo,
        camera: &CameraInfo,
        settings: &RayTracingSettings,
        canvas: &Canvas,
        eid_image: Arc<ImageView>,
    ) -> (Box<dyn GpuFuture>, Arc<PrimaryAutoCommandBuffer>) {
        let mut instances = Vec::new();
        let mut obj_descs = Vec::new();
        let mut texture_resources = IndexSet::new();
        let mut texture_indices = Vec::new();
        let mut materials = Vec::new();
        let mut eids = Vec::new();

        let mesh_blas_future = draw_meshs(
            &canvas.meshs,
            context,
            &mut instances,
            &mut obj_descs,
            &mut texture_resources,
            &mut texture_indices,
            &mut materials,
            &mut eids,
        );

        let circle_blas_future = draw_shapes(
            &canvas.circles,
            1,
            AabbPositions {
                min: [-0.5, -0.5, 0.0],
                max: [0.5, 0.5, 0.0],
            },
            context,
            &mut instances,
            &mut texture_resources,
            &mut texture_indices,
            &mut materials,
            &mut eids,
        );

        // Empty instance vector will make blas creation fail, we avoid this issue by adding an invisble instance.
        // TODO: how to create an empty tlas?
        let spheres = if canvas.spheres.is_empty() {
            &vec![(
                Vec4::ZERO,
                None,
                Material::default(),
                Affine3A::from_scale(Vec3::ZERO),
                EntityId::dead(),
            )]
        } else {
            &canvas.spheres
        };

        let sphere_blas_future = draw_shapes(
            spheres,
            2,
            AabbPositions {
                min: [-0.5, -0.5, -0.5],
                max: [0.5, 0.5, 0.5],
            },
            context,
            &mut instances,
            &mut texture_resources,
            &mut texture_indices,
            &mut materials,
            &mut eids,
        );

        let (tlas, tlas_future) = util::vulkano::create_top_level_acceleration_structure(
            context.memory_allocator.clone(),
            &context.command_buffer_allocator,
            context.graphics_queue.clone(),
            instances,
        );

        let eid_buffer = create_buffer(
            eids.into_iter()
                .flatten()
                .map(|e| crate::render::canvas::eid_to_u32_array(e))
                .collect::<Vec<_>>(),
            &context.memory_allocator,
        );
        let material_buffer = create_buffer(
            materials.into_iter().flatten().collect::<Vec<_>>(),
            &context.memory_allocator,
        );
        let texture_indices_buffer = create_buffer(
            texture_indices.into_iter().flatten().collect::<Vec<_>>(),
            &context.memory_allocator,
        );
        let mut descriptor_writes = vec![
            WriteDescriptorSet::acceleration_structure(0, tlas),
            WriteDescriptorSet::image_view(1, info.image.clone()),
            WriteDescriptorSet::image_view(2, eid_image),
            WriteDescriptorSet::buffer(3, eid_buffer),
            WriteDescriptorSet::buffer(4, material_buffer),
            WriteDescriptorSet::buffer(6, texture_indices_buffer),
        ];
        if !obj_descs.is_empty() {
            let obj_desc_buffer = create_buffer(obj_descs, &context.memory_allocator);
            descriptor_writes.push(WriteDescriptorSet::buffer(5, obj_desc_buffer));
        }
        let texture_resource_count = texture_resources.len();
        if !texture_resources.is_empty() {
            descriptor_writes.push(WriteDescriptorSet::image_view_sampler_array(
                7,
                0,
                texture_resources
                    .into_iter()
                    .map(|t| (t.image_view, t.sampler)),
            ));
        }

        let descriptor_set = PersistentDescriptorSet::new_variable(
            &context.descriptor_set_allocator,
            self.descriptor_set_layout.clone(),
            texture_resource_count as _,
            descriptor_writes,
            [],
        )
        .unwrap();

        let push_constants = shader::raygen::PushConstants {
            camera_type: camera.settings.to_i32() as u32,
            camera_position: camera.position.to_array(),
            camera_direction: camera.direction().to_array(),
            camera_data: match camera.settings {
                CameraSettings::Orthographic { height, .. } => height,
                CameraSettings::Perspective { fov, .. } => fov,
            },
            camera_lens_radius: settings.camera_lens_radius,
            camera_focus_dist: settings.camera_focus_dist,
            samples: settings.samples,
            max_bounces: settings.max_bounces,
            miss_color_top: settings.miss_color_top.to_array(),
            miss_color_bottom: settings.miss_color_bottom.to_array(),
            seed: self.rng.next_u32(),
        };

        let command_buffer = AutoCommandBufferBuilder::primary(
            &context.command_buffer_allocator,
            context.graphics_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap()
        .build()
        .unwrap();

        let command_buffer_handle = command_buffer.handle();
        unsafe {
            context
                .ash
                .device()
                .begin_command_buffer(
                    command_buffer_handle,
                    &vk::CommandBufferBeginInfo::builder()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                        .build(),
                )
                .expect("Failed to begin recording Command Buffer at beginning!");
            context.ash.device().cmd_bind_pipeline(
                command_buffer_handle,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                *self.pipeline,
            );
            context.ash.device().cmd_bind_descriptor_sets(
                command_buffer_handle,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                self.pipeline_layout.handle(),
                0,
                &[descriptor_set.handle()],
                &[],
            );
            context.ash.device().cmd_push_constants(
                command_buffer_handle,
                self.pipeline_layout.handle(),
                vk::ShaderStageFlags::RAYGEN_KHR,
                0,
                std::slice::from_raw_parts(
                    &push_constants as *const shader::raygen::PushConstants as *const u8,
                    std::mem::size_of::<shader::raygen::PushConstants>(),
                ),
            );
            context.ash.rt_pipeline().cmd_trace_rays(
                command_buffer_handle,
                &self.sbt_region.raygen,
                &self.sbt_region.miss,
                &self.sbt_region.hit,
                &self.sbt_region.call,
                info.image.image().extent()[0],
                info.image.image().extent()[1],
                1,
            );
            context
                .ash
                .device()
                .end_command_buffer(command_buffer_handle)
                .unwrap();
        }

        (
            mesh_blas_future
                .join(circle_blas_future)
                .join(sphere_blas_future)
                .join(tlas_future)
                .boxed(),
            command_buffer,
        )
    }
}

fn draw_meshs(
    meshs: &Vec<(
        Arc<MeshData>,
        Vec4,
        Option<TextureData>,
        Material,
        Affine3A,
        EntityId,
    )>,
    context: &RenderContext,
    instances: &mut Vec<(Arc<AccelerationStructure>, u32, Vec<Affine3A>)>,
    obj_descs: &mut Vec<shader::closesthit::ObjDesc>,
    texture_resources: &mut IndexSet<TextureData>,
    texture_indices: &mut Vec<Vec<u32>>,
    materials: &mut Vec<Vec<EnumMaterial>>,
    eids: &mut Vec<Vec<EntityId>>,
) -> Box<dyn GpuFuture> {
    let mut gpu_future = vulkano::sync::now(context.device.clone()).boxed();
    if meshs.is_empty() {
        return gpu_future;
    }

    let mut mesh_to_index = HashMap::new();
    for (mesh, color, texture_data, material, model_matrix, eid) in meshs {
        let i = *mesh_to_index.entry(mesh.clone()).or_insert_with(|| {
            let ((blas, blas_future), (vertex_address, index_address)) =
                util::vulkano::create_bottom_level_acceleration_structure_triangles(
                    context.memory_allocator.clone(),
                    &context.command_buffer_allocator,
                    context.graphics_queue.clone(),
                    mesh.vertices
                        .iter()
                        .map(|v| shader::closesthit::Vertex::new(v.position, v.normal, v.tex_coord))
                        .collect(),
                    Some(mesh.indices.clone()),
                );

            let pre_future = std::mem::replace(
                &mut gpu_future,
                vulkano::sync::now(context.device.clone()).boxed(),
            );
            gpu_future = pre_future.join(blas_future).boxed();

            instances.push((blas, 0, Vec::new()));
            obj_descs.push(shader::closesthit::ObjDesc {
                vertex_address,
                index_address,
            });

            texture_indices.push(Vec::new());
            materials.push(Vec::new());
            eids.push(Vec::new());

            instances.len() - 1
        });

        let transforms = &mut instances[i].2;
        transforms.push(*model_matrix);

        materials[i].push(EnumMaterial::from_material(*material, *color));
        eids[i].push(*eid);

        let texture_index = if let Some(texture_data) = texture_data {
            texture_resources.insert(texture_data.clone());
            texture_resources.get_index_of(texture_data).unwrap() as _
        } else {
            u32::MAX
        };
        texture_indices[i].push(texture_index);
    }

    gpu_future
}

fn draw_shapes(
    shapes: &Vec<(Vec4, Option<TextureData>, Material, Affine3A, EntityId)>,
    sbt_index: u32,
    aabb: AabbPositions,
    context: &RenderContext,
    instances: &mut Vec<(Arc<AccelerationStructure>, u32, Vec<Affine3A>)>,
    texture_resources: &mut IndexSet<TextureData>,
    texture_indices: &mut Vec<Vec<u32>>,
    materials: &mut Vec<Vec<EnumMaterial>>,
    eids: &mut Vec<Vec<EntityId>>,
) -> Box<dyn GpuFuture> {
    if shapes.is_empty() {
        return vulkano::sync::now(context.device.clone()).boxed();
    }

    let mut transforms = Vec::new();
    let i = texture_indices.len();
    texture_indices.push(Vec::new());
    materials.push(Vec::new());
    eids.push(Vec::new());
    for (color, texture_data, material, model, eid) in shapes {
        transforms.push(*model);
        let texture_index = if let Some(texture_data) = texture_data {
            texture_resources.insert(texture_data.clone());
            texture_resources.get_index_of(texture_data).unwrap() as _
        } else {
            u32::MAX
        };
        texture_indices[i].push(texture_index);
        materials[i].push(EnumMaterial::from_material(*material, *color));
        eids[i].push(*eid);
    }

    let (blas, blas_future) = util::vulkano::create_bottom_level_acceleration_structure_aabbs(
        context.memory_allocator.clone(),
        &context.command_buffer_allocator,
        context.graphics_queue.clone(),
        vec![aabb],
    );

    instances.push((blas, sbt_index, transforms));

    blas_future
}

fn create_buffer<T: BufferContents>(
    iter: impl IntoIterator<Item = T, IntoIter: ExactSizeIterator>,
    memory_allocator: &Arc<StandardMemoryAllocator>,
) -> vulkano::buffer::Subbuffer<[T]> {
    Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        iter,
    )
    .unwrap()
}
