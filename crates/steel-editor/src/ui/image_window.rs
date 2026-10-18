use egui_winit_vulkano::Gui;
use glam::{UVec2, Vec2};
use std::sync::Arc;
use vulkano::{
    command_buffer::{
        allocator::CommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        CopyImageInfo, PrimaryCommandBufferAbstract,
    },
    device::Queue,
    format::Format,
    image::{view::ImageView, Image, ImageCreateInfo, ImageUsage},
    memory::allocator::AllocationCreateInfo,
    sync::GpuFuture,
};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};

/// An egui window which displays a image.
pub struct ImageWindow {
    title: String,
    image_index: usize,
    /// We need 2 images every frame because [egui_winit_vulkano] regard all texture input as in linear color space.
    /// The first image is the unorm image of gamma color space that we will draw,
    /// the second image is the srgb image that the first image will copy to and is given to [egui_winit_vulkano] to have linear color input.
    /// We could have solved this problem by creating an unorm image view and a srgb image view for only one unorm image,
    /// so that we could draw on unorm image view and give srgb image view to [egui_winit_vulkano] to have linear color input.
    /// However the ray tracing pipeline requires an image with storage usage, which cannot have an image view of srgb format.
    /// TODO: how to avoid copying image every frame?
    images: Option<Vec<(Arc<ImageView>, Arc<ImageView>)>>,
    texture_ids: Option<Vec<egui::TextureId>>,
    pixel: UVec2,
    size: Vec2,
    position: Vec2,
}

impl ImageWindow {
    pub fn new(title: impl Into<String>) -> Self {
        ImageWindow {
            title: title.into(),
            image_index: 0,
            images: None,
            texture_ids: None,
            pixel: UVec2::ZERO,
            size: Vec2::ZERO,
            position: Vec2::ZERO,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        gui: &mut Gui,
        context: &VulkanoContext,
        renderer: &VulkanoWindowRenderer,
    ) {
        self.image_index = renderer.image_index() as usize;
        let available_size = ui.available_size();
        (self.size.x, self.size.y) = (available_size.x, available_size.y);
        let pixel = (self.size * ui.ctx().pixels_per_point()).as_uvec2();
        if self.images.is_none() || self.pixel.x != pixel.x || self.pixel.y != pixel.y {
            self.pixel = pixel;
            self.close(Some(gui));
            self.images = Some(
                (0..renderer.swapchain_image_views().len())
                    .map(|_| {
                        let unorm_image = Image::new(
                            context.memory_allocator().clone(),
                            ImageCreateInfo {
                                format: renderer.swapchain_format(), // unorm
                                extent: [self.pixel.x, self.pixel.y, 1],
                                usage: ImageUsage::TRANSFER_SRC
                                    | ImageUsage::SAMPLED
                                    | ImageUsage::STORAGE
                                    | ImageUsage::COLOR_ATTACHMENT,
                                ..Default::default()
                            },
                            AllocationCreateInfo::default(),
                        )
                        .unwrap();
                        let srgb_image = Image::new(
                            context.memory_allocator().clone(),
                            ImageCreateInfo {
                                format: Format::B8G8R8A8_SRGB,
                                extent: [self.pixel.x, self.pixel.y, 1],
                                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                                ..Default::default()
                            },
                            AllocationCreateInfo::default(),
                        )
                        .unwrap();
                        (
                            ImageView::new_default(unorm_image).unwrap(),
                            ImageView::new_default(srgb_image).unwrap(),
                        )
                    })
                    .collect(),
            );
            self.texture_ids = Some(
                self.images
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|(_, srgb_image)| {
                        gui.register_user_image_view(srgb_image.clone(), Default::default())
                    })
                    .collect(),
            );
            log::trace!(
                "ImageWindow({}): image created, pixel={}, size={}",
                self.title,
                self.pixel,
                self.size
            );
        }
        let texture_id = self.texture_ids.as_ref().unwrap()[self.image_index];
        let r = ui.image(egui::ImageSource::Texture(egui::load::SizedTexture::new(
            texture_id,
            available_size,
        )));
        (self.position.x, self.position.y) = (r.rect.left(), r.rect.top());
    }

    pub fn close(&mut self, gui: Option<&mut Gui>) {
        self.images = None;
        if let (Some(gui), Some(texture_ids)) = (gui, &self.texture_ids) {
            for texture_id in texture_ids {
                gui.unregister_user_image(*texture_id);
            }
        }
        self.texture_ids = None;
    }

    /// Get window image of current frame to draw, return None if images are not created yet.
    pub fn image(&self) -> Option<&Arc<ImageView>> {
        self.images
            .as_ref()
            .and_then(|images| images.get(self.image_index))
            .map(|(unorm_image, _)| unorm_image)
    }

    /// Get the exact pixel of window images
    pub fn pixel(&self) -> UVec2 {
        self.pixel
    }

    /// Get the window size which is scaled by window scale factor
    pub fn size(&self) -> Vec2 {
        self.size
    }

    /// Get the window position which is scaled by window scale factor
    pub fn position(&self) -> Vec2 {
        self.position
    }

    /// copy unorm image to srgb image of current frame.
    pub fn copy_image(
        &self,
        allocator: Arc<dyn CommandBufferAllocator>,
        queue: Arc<Queue>,
        befor_future: Box<dyn GpuFuture>,
    ) -> Box<dyn GpuFuture> {
        let (unorm_image, srgb_image) = &self.images.as_ref().unwrap()[self.image_index];
        let mut builder = AutoCommandBufferBuilder::primary(
            allocator,
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
        builder
            .copy_image(CopyImageInfo::images(
                unorm_image.image().clone(),
                srgb_image.image().clone(),
            ))
            .unwrap();
        builder
            .build()
            .unwrap()
            .execute_after(befor_future, queue)
            .unwrap()
            .boxed()
    }
}
