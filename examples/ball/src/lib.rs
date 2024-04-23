use steel::engine::{Engine, EngineImpl};

#[no_mangle]
pub fn create() -> Box<dyn Engine> {
    Box::new(EngineImpl::new())
}
