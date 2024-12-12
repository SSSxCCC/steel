pub mod ash;
pub mod vulkano;

use ::ash::vk;
use ::vulkano::{
    pipeline::PipelineShaderStageCreateInfo,
    shader::{spirv::ExecutionModel, ShaderModule},
    VulkanObject,
};
use glam::Affine3A;
use std::sync::Arc;

pub fn affine3a_to_rows_array_2d(affine: Affine3A) -> [[f32; 4]; 3] {
    let cols = affine.to_cols_array_2d(); // [[f32; 3]; 4] column-major
    [
        [cols[0][0], cols[1][0], cols[2][0], cols[3][0]], // First row
        [cols[0][1], cols[1][1], cols[2][1], cols[3][1]], // Second row
        [cols[0][2], cols[1][2], cols[2][2], cols[3][2]], // Third row
    ]
}

/// Create both ash and vulkano shader stages.
pub fn create_shader_stages(
    shader_modules: impl IntoIterator<Item = Arc<ShaderModule>>,
) -> (
    Vec<vk::PipelineShaderStageCreateInfo>,
    Vec<PipelineShaderStageCreateInfo>,
) {
    shader_modules
        .into_iter()
        .map(|shader_module| {
            let entry_point = shader_module.entry_point("main").unwrap();
            (
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(match entry_point.info().execution_model {
                        ExecutionModel::RayGenerationKHR => vk::ShaderStageFlags::RAYGEN_KHR,
                        ExecutionModel::IntersectionKHR => vk::ShaderStageFlags::INTERSECTION_KHR,
                        ExecutionModel::AnyHitKHR => vk::ShaderStageFlags::ANY_HIT_KHR,
                        ExecutionModel::ClosestHitKHR => vk::ShaderStageFlags::CLOSEST_HIT_KHR,
                        ExecutionModel::MissKHR => vk::ShaderStageFlags::MISS_KHR,
                        ExecutionModel::CallableKHR => vk::ShaderStageFlags::CALLABLE_KHR,
                        _ => panic!("Unknown stage: {:?}", entry_point.info().execution_model),
                    })
                    .module(shader_module.handle())
                    .name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap())
                    .build(),
                PipelineShaderStageCreateInfo::new(entry_point),
            )
        })
        .unzip()
}
