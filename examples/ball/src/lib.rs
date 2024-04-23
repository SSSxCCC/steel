use rapier2d::prelude::*;
use shipyard::{Component, IntoIter, UniqueViewMut, View};
use steel::{edit::Edit, engine::{Command, DrawInfo, Engine, EngineImpl, FrameInfo, FrameStage, InitInfo}, physics2d::{Physics2DManager, RigidBody2D}};
use vulkano::sync::GpuFuture;

#[no_mangle]
pub fn create() -> Box<dyn Engine> {
    Box::new(EngineWrapper { inner: EngineImpl::new() })
}

struct EngineWrapper {
    inner: EngineImpl,
}

impl Engine for EngineWrapper {
    fn init(&mut self, info: InitInfo) {
        self.inner.init(info);
        self.inner.register_component::<Player>();
    }

    fn frame(&mut self, info: &FrameInfo) {
        self.inner.frame(info);
        match info.stage {
            FrameStage::Maintain => (),
            FrameStage::Update => {
                self.inner.world.run(|player: View<Player>, rb2d: View<RigidBody2D>, mut physics2d_manager: UniqueViewMut<Physics2DManager>| {
                    for (_, rb2d) in (&player, &rb2d).iter() {
                        if let Some(rb2d) = physics2d_manager.rigid_body_set.get_mut(rb2d.handle()) {
                            rb2d.set_linvel(vector![1.0, 0.0], true);
                        }
                    }
                });
            },
            FrameStage::Finish => (),
        }
    }

    fn draw(&mut self, info: DrawInfo) -> Box<dyn GpuFuture> {
        self.inner.draw(info)
    }

    fn command(&mut self, cmd: Command) {
        self.inner.command(cmd);
    }
}

#[derive(Component, Default)]
pub struct Player;

impl Edit for Player {
    fn name() -> &'static str { "Player" }
}
