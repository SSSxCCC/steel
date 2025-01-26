//! The [steel game engine](https://github.com/SSSxCCC/steel) common library, depended by both steel-engine and steel-editor.

pub mod app;
pub mod asset;
pub mod camera;
pub mod data;
pub mod ext;
pub mod platform;
pub mod prefab;

use vulkano::{
    device::{DeviceExtensions, Features},
    instance::{Instance, InstanceCreateInfo},
    VulkanLibrary,
};
use vulkano_util::context::{VulkanoConfig, VulkanoContext};

/// Helper function to create vulkano context and enable ray tracing extensions and features if supported.
/// Returns [VulkanoContext] and a bool indicating whether ray tracing is enabled. This is used by steel-editor and steel-client.
pub fn create_context() -> (VulkanoContext, bool) {
    let mut config = VulkanoConfig::default();
    config.device_features.fill_mode_non_solid = true;
    config.device_features.independent_blend = true;
    config.device_features.runtime_descriptor_array = true;
    config
        .device_features
        .descriptor_binding_variable_descriptor_count = true;
    config.device_features.shader_int64 = true;

    let library = VulkanLibrary::new().expect("failed to load Vulkan library");
    let instance =
        Instance::new(library, InstanceCreateInfo::default()).expect("failed to create instance");
    let physical_device = instance
        .enumerate_physical_devices()
        .expect("failed to enumerate physical devices")
        .filter(|p| (config.device_filter_fn)(p))
        .min_by_key(|p| (config.device_priority_fn)(p))
        .expect("failed to create physical device");

    let mut ray_tracing_extensions = DeviceExtensions::empty();
    ray_tracing_extensions.khr_deferred_host_operations = true;
    ray_tracing_extensions.khr_acceleration_structure = true;
    ray_tracing_extensions.khr_ray_tracing_pipeline = true;
    let mut ray_tracing_features = Features::empty();
    ray_tracing_features.acceleration_structure = true;
    ray_tracing_features.ray_tracing_pipeline = true;
    ray_tracing_features.buffer_device_address = true;
    let ray_tracing_supported = physical_device
        .supported_extensions()
        .contains(&ray_tracing_extensions)
        && physical_device
            .supported_features()
            .contains(&ray_tracing_features);

    if ray_tracing_supported {
        config.device_extensions = config.device_extensions.union(&ray_tracing_extensions);
        config.device_features = config.device_features.union(&ray_tracing_features);
    }
    let context = VulkanoContext::new(config);
    log::info!(
        "Physical device: {}, ray tracing supported: {}",
        context.device().physical_device().properties().device_name,
        ray_tracing_supported
    );

    (context, ray_tracing_supported)
}
