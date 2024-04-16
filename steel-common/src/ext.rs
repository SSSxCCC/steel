use vulkano_util::renderer::VulkanoWindowRenderer;

/// Currently we can't get the count of swapchain images from VulkanoWindowRenderer,
/// VulkanoWindowRendererExt trait supplies a temporary funtion to get the count.
/// TODO: remove this once we can get count from VulkanoWindowRenderer.
pub trait VulkanoWindowRendererExt {
    fn image_count(&self) -> usize;
}

impl VulkanoWindowRendererExt for VulkanoWindowRenderer {
    /// Currently we can't get the count of swapchain images from VulkanoWindowRenderer,
    /// This is a temporary funtion to get the count.
    /// TODO: remove this once we can get count from VulkanoWindowRenderer.
    fn image_count(&self) -> usize {
        2
    }
}
