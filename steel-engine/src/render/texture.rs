use super::{image::ImageAssets, RenderContext};
use crate::asset::AssetManager;
use image::{DynamicImage, GenericImageView};
use shipyard::Unique;
use std::{collections::HashMap, error::Error, sync::Arc};
use steel_common::{asset::AssetId, platform::Platform};
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo,
        PrimaryCommandBufferAbstract,
    },
    format::Format,
    image::{
        sampler::{Sampler, SamplerCreateInfo},
        view::ImageView,
        Image, ImageCreateInfo, ImageUsage,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    sync::GpuFuture,
};

struct TextureAsset {
    image: Arc<DynamicImage>,
    data: (Arc<ImageView>, Arc<Sampler>),
}

#[derive(Unique, Default)]
/// Cache [ImageView] and [Sampler] in assets.
pub struct TextureAssets {
    textures: HashMap<AssetId, TextureAsset>,
}

impl TextureAssets {
    pub fn get_texture(
        &mut self,
        asset_id: AssetId,
        image_assets: &mut ImageAssets,
        asset_manager: &mut AssetManager,
        platform: &Platform,
        render_context: &RenderContext,
    ) -> Option<(Arc<ImageView>, Arc<Sampler>)> {
        if let Some(image) = image_assets.get_image(asset_id, asset_manager, platform) {
            if let Some(texture2d_asset) = self.textures.get(&asset_id) {
                if Arc::ptr_eq(&image, &texture2d_asset.image) {
                    // cache is still valid
                    return Some(texture2d_asset.data.clone());
                }
            }
            // cache is not valid, reload data
            match Self::get_texture_from_image(&image, render_context) {
                Ok(data) => {
                    self.textures.insert(
                        asset_id,
                        TextureAsset {
                            image: image.clone(),
                            data: data.clone(),
                        },
                    );
                    return Some(data);
                }
                Err(e) => log::error!("Texture2DAssets::get_texture: error: {}", e),
            }
        }
        self.textures.remove(&asset_id);
        None
    }

    fn get_texture_from_image(
        dynamic_image: &DynamicImage,
        render_context: &RenderContext,
    ) -> Result<(Arc<ImageView>, Arc<Sampler>), Box<dyn Error>> {
        let image_staging_buffer = Buffer::new_slice(
            render_context.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE
                    | MemoryTypeFilter::PREFER_HOST,
                ..Default::default()
            },
            (dynamic_image.dimensions().0 * dynamic_image.dimensions().1) as u64 * 4,
        )?;
        image_staging_buffer
            .write()?
            .copy_from_slice(&dynamic_image.to_rgba8());
        let image = Image::new(
            render_context.memory_allocator.clone(),
            ImageCreateInfo {
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                format: Format::R8G8B8A8_SRGB,
                extent: [
                    dynamic_image.dimensions().0,
                    dynamic_image.dimensions().1,
                    1,
                ],
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )?;
        let image_view = ImageView::new_default(image.clone())?;
        let sampler = Sampler::new(
            render_context.device.clone(),
            SamplerCreateInfo::simple_repeat_linear_no_mipmap(),
        )?;
        let mut upload_image_commnad_buffer = AutoCommandBufferBuilder::primary(
            &render_context.command_buffer_allocator,
            render_context.graphics_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )?;
        upload_image_commnad_buffer.copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
            image_staging_buffer,
            image.clone(),
        ))?;
        upload_image_commnad_buffer
            .build()?
            .execute(render_context.graphics_queue.clone())?
            .then_signal_fence_and_flush()?
            .wait(None)?;
        Ok((image_view, sampler))
    }
}
