use shipyard::Unique;
use winit_input_helper::WinitInputHelper;

/// The Input contains inputs happened in this frame, it is just a wrapper of [winit_input_helper::WinitInputHelper].
#[derive(Unique)]
pub struct Input(WinitInputHelper);

impl Input {
    pub fn new() -> Self {
        Input(WinitInputHelper::new())
    }
}

impl std::ops::Deref for Input {
    type Target = WinitInputHelper;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Input {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
