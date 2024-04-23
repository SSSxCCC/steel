use steel::engine::{Command, DrawInfo, Engine, EngineImpl, InitInfo, FrameInfo};
use vulkano::sync::GpuFuture;

#[no_mangle]
pub fn create() -> Box<dyn Engine> {
    Box::new(EngineImpl::new())
}

struct EngineWrapper {
    inner: EngineImpl,
}

impl Engine for EngineWrapper {
    fn init(&mut self, info: InitInfo) {
        self.inner.init(info);
    }

    fn frame(&mut self, info: &FrameInfo) {
        self.inner.frame(info);
    }

    fn draw(&mut self, info: DrawInfo) -> Box<dyn GpuFuture> {
        self.inner.draw(info)
    }

    fn command(&mut self, cmd: Command) {
        self.inner.command(cmd);
    }
}
