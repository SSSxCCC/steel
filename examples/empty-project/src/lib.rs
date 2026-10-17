use steel::{app::SteelApp, log, steel_main};

#[steel_main]
fn main() {
    SteelApp::new()
        .add_system(steel::app::Schedule::Init, 0, hello_world_system)
        .run();
}

fn hello_world_system() {
    log::info!("Hello world!");
}
