use egui_winit_vulkano::Gui;
use glam::{UVec2, Vec2};
use std::sync::Arc;
use steel_common::ext::VulkanoWindowRendererExt;
use vulkano::{
    image::{view::ImageView, Image, ImageCreateInfo, ImageUsage},
    memory::allocator::AllocationCreateInfo,
};
use vulkano_util::{context::VulkanoContext, renderer::VulkanoWindowRenderer};

/// A egui window which displays a image
pub struct ImageWindow {
    title: String,
    image_index: usize,
    images: Option<Vec<Arc<ImageView>>>,
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
                (0..renderer.image_count())
                    .map(|_| {
                        let image = Image::new(
                            context.memory_allocator().clone(),
                            ImageCreateInfo {
                                format: renderer.swapchain_format(),
                                extent: [self.pixel.x, self.pixel.y, 1],
                                usage: ImageUsage::SAMPLED | ImageUsage::COLOR_ATTACHMENT,
                                ..Default::default()
                            },
                            AllocationCreateInfo::default(),
                        )
                        .unwrap();
                        ImageView::new_default(image).unwrap()
                    })
                    .collect(),
            );
            self.texture_ids = Some(
                self.images
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|image| gui.register_user_image_view(image.clone(), Default::default()))
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

    /// Get window image of current frame, return None if images are not created yet.
    pub fn image(&self) -> Option<&Arc<ImageView>> {
        self.images
            .as_ref()
            .and_then(|images| images.get(self.image_index))
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
}
