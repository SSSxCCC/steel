use steel::app::{App, SteelApp};

#[no_mangle]
pub fn create() -> Box<dyn App> {
    SteelApp::new().boxed()
}
