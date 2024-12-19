pub mod material;

pub(crate) mod util;

mod shader;

use crate::{
    camera::CameraInfo,
    render::{canvas::Canvas, FrameRenderInfo, RenderContext},
};
use ash::vk;
use bytemuck::{Pod, Zeroable};
use glam::{Affine3A, Vec3, Vec4, Vec4Swizzles};
use material::Material;
use rand::{rngs::StdRng, RngCore, SeedableRng};
use std::sync::Arc;
use steel_common::{
    camera::CameraSettings,
    data::{Data, Limit, Value},
};
use util::ash::{AshBuffer, AshPipeline, SbtRegion, ShaderGroup};
use vulkano::{
    acceleration_structure::AabbPositions,
    buffer::{Buffer, BufferCreateInfo, BufferUsage},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer},
    descriptor_set::{layout::DescriptorSetLayout, PersistentDescriptorSet, WriteDescriptorSet},
    device::Device,
    image::view::ImageView,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{layout::PipelineDescriptorSetLayoutCreateInfo, PipelineLayout},
    sync::GpuFuture,
    VulkanObject,
};

#[derive(Clone, Copy, Default, Zeroable, Pod)]
#[repr(C)]
pub(crate) struct EnumMaterialPod {
    data: [f32; 4],
    t: u32,
    _pad: [f32; 3],
}

impl EnumMaterialPod {
    pub fn new_lambertian(albedo: Vec3) -> Self {
        Self {
            data: [albedo.x, albedo.y, albedo.z, 0.0],
            t: 0,
            _pad: [0.0, 0.0, 0.0],
        }
    }

    pub fn new_metal(albedo: Vec3, fuzz: f32) -> Self {
        Self {
            data: [albedo.x, albedo.y, albedo.z, fuzz],
            t: 1,
            _pad: [0.0, 0.0, 0.0],
        }
    }

    pub fn new_dielectric(ri: f32) -> Self {
        Self {
            data: [ri, 0.0, 0.0, 0.0],
            t: 2,
            _pad: [0.0, 0.0, 0.0],
        }
    }

    pub fn from_material(material: Material, color: Vec4) -> Self {
        let color = color.xyz() * color.w;
        match material {
            Material::Lambertian => Self::new_lambertian(color),
            Material::Metal { fuzz } => Self::new_metal(color, fuzz),
            Material::Dielectric { ri } => Self::new_dielectric(ri), // TODO: color
        }
    }
}

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
            miss_color_top: Vec3::ZERO,
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
        let sphere_intersection_shader_module =
            shader::sphere_intersection::load(context.device.clone()).unwrap();
        let sphere_closesthit_shader_module =
            shader::sphere_closesthit::load(context.device.clone()).unwrap();

        let (shader_stages, stages) = util::create_shader_stages([
            raygen_shader_module,
            miss_shader_module,
            sphere_intersection_shader_module,
            sphere_closesthit_shader_module,
        ]);

        let shader_groups = util::ash::create_shader_groups([
            ShaderGroup::General(0),
            ShaderGroup::General(1),
            ShaderGroup::ProceduralHitGroup {
                closest_hit_shader: 3,
                any_hit_shader: vk::SHADER_UNUSED_KHR,
                intersection_shader: 2,
            },
        ]);

        let pipeline_layout = PipelineLayout::new(
            context.device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
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
        let mut sphere_instances = Vec::new();
        let mut materials = Vec::new();
        for (model, color, material, eid) in &canvas.spheres {
            sphere_instances.push(*model);
            materials.push(EnumMaterialPod::from_material(*material, *color));
        }
        // TODO: how to create an empty tlas?
        if sphere_instances.is_empty() {
            sphere_instances.push(Affine3A::from_scale(Vec3::ZERO));
            materials.push(EnumMaterialPod::new_lambertian(Vec3::ZERO));
        }

        let (blas, blas_future) = util::vulkano::create_aabb_bottom_level_acceleration_structure(
            context.memory_allocator.clone(),
            &context.command_buffer_allocator,
            context.graphics_queue.clone(),
            vec![AabbPositions {
                min: [-1.0, -1.0, -1.0],
                max: [1.0, 1.0, 1.0],
            }],
        );

        let (tlas, tlas_future) = util::vulkano::create_top_level_acceleration_structure(
            context.memory_allocator.clone(),
            &context.command_buffer_allocator,
            context.graphics_queue.clone(),
            vec![(blas, 0, sphere_instances)],
        );

        let material_buffer = Buffer::from_iter(
            context.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            materials,
        )
        .unwrap();

        let descriptor_set = PersistentDescriptorSet::new(
            &context.descriptor_set_allocator,
            self.descriptor_set_layout.clone(),
            [
                WriteDescriptorSet::acceleration_structure(0, tlas),
                WriteDescriptorSet::image_view(1, info.image.clone()),
                WriteDescriptorSet::buffer(2, material_buffer),
            ],
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

        (blas_future.join(tlas_future).boxed(), command_buffer)
    }
}
