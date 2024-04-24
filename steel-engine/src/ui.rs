use shipyard::Unique;

/// EguiContext is a wrapper of egui::Context, you can use this unique to show your ui.
/// EguiContext is added in EngineImpl::maintain and is removed in EngineImpl::finish.
#[derive(Unique)]
pub struct EguiContext(egui::Context);

impl EguiContext {
    pub fn new(ctx: egui::Context) -> Self {
        EguiContext(ctx)
    }
}

impl std::ops::Deref for EguiContext {
    type Target = egui::Context;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
