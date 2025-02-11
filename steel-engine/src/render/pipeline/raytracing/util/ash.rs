use ash::{extensions::khr::RayTracingPipeline, util::Align, vk, Device, Entry, Instance};
use std::sync::Arc;
use vulkano::VulkanObject;
use vulkano_util::context::VulkanoContext;

pub struct AshContext {
    #[allow(unused)]
    entry: Arc<Entry>,
    #[allow(unused)]
    instance: Arc<Instance>,
    device: Arc<Device>,
    rt_pipeline: RayTracingPipeline,
    rt_pipeline_properties: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
    device_memory_properties: vk::PhysicalDeviceMemoryProperties,
}

unsafe impl Send for AshContext {}
unsafe impl Sync for AshContext {}

impl AshContext {
    pub fn new(vulkano: &VulkanoContext) -> Self {
        let entry = Arc::new(unsafe { ash::Entry::load() }.unwrap());
        let instance = Arc::new(unsafe {
            ash::Instance::load(entry.static_fn(), vulkano.instance().handle())
        });
        let device =
            Arc::new(unsafe { ash::Device::load(instance.fp_v1_0(), vulkano.device().handle()) });
        let rt_pipeline = RayTracingPipeline::new(&instance, &device);
        let mut rt_pipeline_properties =
            vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::default();
        {
            let mut physical_device_properties2 = vk::PhysicalDeviceProperties2::builder()
                .push_next(&mut rt_pipeline_properties)
                .build();
            unsafe {
                instance.get_physical_device_properties2(
                    vulkano.device().physical_device().handle(),
                    &mut physical_device_properties2,
                );
            }
        }
        let device_memory_properties = unsafe {
            instance
                .get_physical_device_memory_properties(vulkano.device().physical_device().handle())
        };
        Self {
            entry,
            instance,
            device,
            rt_pipeline,
            rt_pipeline_properties,
            device_memory_properties,
        }
    }

    #[allow(unused)]
    pub fn entry(&self) -> &Arc<Entry> {
        &self.entry
    }

    #[allow(unused)]
    pub fn instance(&self) -> &Arc<Instance> {
        &self.instance
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }

    pub fn rt_pipeline(&self) -> &RayTracingPipeline {
        &self.rt_pipeline
    }

    pub fn rt_pipeline_properties(&self) -> &vk::PhysicalDeviceRayTracingPipelinePropertiesKHR {
        &self.rt_pipeline_properties
    }

    pub fn device_memory_properties(&self) -> vk::PhysicalDeviceMemoryProperties {
        self.device_memory_properties
    }
}

#[derive(Clone)]
pub struct AshBuffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    size: vk::DeviceSize,
    device: Arc<ash::Device>,
}

impl AshBuffer {
    pub fn new(
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        memory_properties: vk::MemoryPropertyFlags,
        device: &Arc<ash::Device>,
        device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    ) -> Self {
        unsafe {
            let buffer_info = vk::BufferCreateInfo::builder()
                .size(size)
                .usage(usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .build();

            let buffer = device.create_buffer(&buffer_info, None).unwrap();

            let memory_req = device.get_buffer_memory_requirements(buffer);

            let memory_index = Self::get_memory_type_index(
                device_memory_properties,
                memory_req.memory_type_bits,
                memory_properties,
            );

            let mut memory_allocate_flags_info = vk::MemoryAllocateFlagsInfo::builder()
                .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS)
                .build();

            let mut allocate_info_builder = vk::MemoryAllocateInfo::builder();

            if usage.contains(vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS) {
                allocate_info_builder =
                    allocate_info_builder.push_next(&mut memory_allocate_flags_info);
            }

            let allocate_info = allocate_info_builder
                .allocation_size(memory_req.size)
                .memory_type_index(memory_index)
                .build();

            let memory = device.allocate_memory(&allocate_info, None).unwrap();

            device.bind_buffer_memory(buffer, memory, 0).unwrap();

            AshBuffer {
                buffer,
                memory,
                size,
                device: device.clone(),
            }
        }
    }

    fn get_memory_type_index(
        device_memory_properties: vk::PhysicalDeviceMemoryProperties,
        mut type_bits: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> u32 {
        for i in 0..device_memory_properties.memory_type_count {
            if (type_bits & 1) == 1
                && (device_memory_properties.memory_types[i as usize].property_flags & properties)
                    == properties
            {
                return i;
            }
            type_bits >>= 1;
        }
        0
    }

    pub fn store<T: Copy>(&mut self, data: &[T], device: &ash::Device) {
        unsafe {
            let size = std::mem::size_of_val(data) as u64;
            assert!(self.size >= size, "Data size is larger than buffer size.");
            let mapped_ptr = self.map(size, device);
            let mut mapped_slice = Align::new(mapped_ptr, std::mem::align_of::<T>() as u64, size);
            mapped_slice.copy_from_slice(data);
            self.unmap(device);
        }
    }

    fn map(&mut self, size: vk::DeviceSize, device: &ash::Device) -> *mut std::ffi::c_void {
        unsafe {
            let data: *mut std::ffi::c_void = device
                .map_memory(self.memory, 0, size, vk::MemoryMapFlags::empty())
                .unwrap();
            data
        }
    }

    fn unmap(&mut self, device: &ash::Device) {
        unsafe {
            device.unmap_memory(self.memory);
        }
    }

    pub fn buffer(&self) -> vk::Buffer {
        self.buffer
    }
}

impl Drop for AshBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

pub enum ShaderGroup {
    General(u32),
    TrianglesHitGroup {
        closest_hit_shader: u32,
        any_hit_shader: u32,
    },
    ProceduralHitGroup {
        closest_hit_shader: u32,
        any_hit_shader: u32,
        intersection_shader: u32,
    },
}

pub fn create_shader_groups(
    groups: impl IntoIterator<Item = ShaderGroup>,
) -> Vec<vk::RayTracingShaderGroupCreateInfoKHR> {
    groups
        .into_iter()
        .map(|group| match group {
            ShaderGroup::General(general_shader) => {
                vk::RayTracingShaderGroupCreateInfoKHR::builder()
                    .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                    .general_shader(general_shader)
                    .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                    .any_hit_shader(vk::SHADER_UNUSED_KHR)
                    .intersection_shader(vk::SHADER_UNUSED_KHR)
                    .build()
            }
            ShaderGroup::TrianglesHitGroup {
                closest_hit_shader,
                any_hit_shader,
            } => vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP)
                .general_shader(vk::SHADER_UNUSED_KHR)
                .closest_hit_shader(closest_hit_shader)
                .any_hit_shader(any_hit_shader)
                .intersection_shader(vk::SHADER_UNUSED_KHR)
                .build(),
            ShaderGroup::ProceduralHitGroup {
                closest_hit_shader,
                any_hit_shader,
                intersection_shader,
            } => vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::PROCEDURAL_HIT_GROUP)
                .general_shader(vk::SHADER_UNUSED_KHR)
                .closest_hit_shader(closest_hit_shader)
                .any_hit_shader(any_hit_shader)
                .intersection_shader(intersection_shader)
                .build(),
        })
        .collect()
}

pub struct AshPipeline {
    pipeline: vk::Pipeline,
    device: Arc<Device>,
}

impl AshPipeline {
    pub fn new(pipeline: vk::Pipeline, device: Arc<Device>) -> Self {
        AshPipeline { pipeline, device }
    }
}

impl std::ops::Deref for AshPipeline {
    type Target = vk::Pipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl Drop for AshPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}

pub struct SbtRegion {
    pub raygen: vk::StridedDeviceAddressRegionKHR,
    pub miss: vk::StridedDeviceAddressRegionKHR,
    pub hit: vk::StridedDeviceAddressRegionKHR,
    pub call: vk::StridedDeviceAddressRegionKHR,
}

pub fn create_sbt_buffer_and_region(
    ash: &AshContext,
    pipeline: vk::Pipeline,
    group_count: usize,
) -> (AshBuffer, SbtRegion) {
    assert!(group_count >= 3);

    let handle_size_aligned = aligned_size(
        ash.rt_pipeline_properties().shader_group_handle_size,
        ash.rt_pipeline_properties().shader_group_base_alignment,
    ) as usize;

    let incoming_table_data = unsafe {
        ash.rt_pipeline().get_ray_tracing_shader_group_handles(
            pipeline,
            0,
            group_count as u32,
            group_count * ash.rt_pipeline_properties().shader_group_handle_size as usize,
        )
    }
    .unwrap();

    let table_size = group_count * handle_size_aligned;
    let mut table_data = vec![0u8; table_size];

    for i in 0..group_count {
        table_data[i * handle_size_aligned
            ..i * handle_size_aligned
                + ash.rt_pipeline_properties().shader_group_handle_size as usize]
            .copy_from_slice(
                &incoming_table_data[i * ash.rt_pipeline_properties().shader_group_handle_size
                    as usize
                    ..(i + 1) * ash.rt_pipeline_properties().shader_group_handle_size as usize],
            );
    }

    let mut shader_binding_table_buffer = AshBuffer::new(
        table_size as u64,
        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::TRANSFER_SRC
            | vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR,
        vk::MemoryPropertyFlags::HOST_VISIBLE, // TODO: DEVICE_LOCAL
        ash.device(),
        ash.device_memory_properties(),
    );
    shader_binding_table_buffer.store(&table_data, ash.device());

    let sbt_address = unsafe {
        ash.device().get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder()
                .buffer(shader_binding_table_buffer.buffer())
                .build(),
        )
    };
    let handle_size_aligned = handle_size_aligned as u64;
    let sbt_region = SbtRegion {
        raygen: vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(sbt_address)
            .size(handle_size_aligned)
            .stride(handle_size_aligned)
            .build(),
        miss: vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(sbt_address + handle_size_aligned)
            .size(handle_size_aligned)
            .stride(handle_size_aligned)
            .build(),
        hit: vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(sbt_address + 2 * handle_size_aligned)
            .size((group_count as u64 - 2) * handle_size_aligned)
            .stride(handle_size_aligned)
            .build(),
        call: vk::StridedDeviceAddressRegionKHR::default(),
    };

    (shader_binding_table_buffer, sbt_region)
}

fn aligned_size(value: u32, alignment: u32) -> u32 {
    (value + alignment - 1) & !(alignment - 1)
}
