use glam::Vec2;
use shipyard::{Component, IntoIter, UniqueView, UniqueViewMut, View, ViewMut};
use steel::{data::{Data, Limit, Value}, edit::Edit, engine::{Command, DrawInfo, Engine, EngineImpl, FrameInfo, FrameStage, InitInfo}, input::Input, physics2d::{Physics2DManager, RigidBody2D}, transform::Transform};
use vulkano::sync::GpuFuture;
use winit::event::VirtualKeyCode;

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
        self.inner.register_component::<Ball>();
    }

    fn frame(&mut self, info: &FrameInfo) {
        self.inner.frame(info);
        match info.stage {
            FrameStage::Maintain => (),
            FrameStage::Update => {
                self.inner.world.run(player_control_system);
                self.inner.world.run(push_ball_system);
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
struct Player {
    move_speed: f32,
}

impl Edit for Player {
    fn name() -> &'static str { "Player" }

    fn get_data(&self) -> Data {
        Data::new().insert("move_speed", Value::Float32(self.move_speed))
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Float32(f)) = data.get("move_speed") { self.move_speed = *f; }
    }
}

fn player_control_system(player: View<Player>, mut transform: ViewMut<Transform>, rb2d: View<RigidBody2D>, mut physics2d_manager: UniqueViewMut<Physics2DManager>, input: UniqueView<Input>) {
    for (player, mut transform, rb2d) in (&player, &mut transform, &rb2d).iter() {
        if let Some(rb2d) = physics2d_manager.rigid_body_set.get_mut(rb2d.handle()) {
            if input.key_held(VirtualKeyCode::Left) {
                rb2d.set_linvel(Vec2::new(-player.move_speed, 0.0).into(), true);
            } else if input.key_held(VirtualKeyCode::Right) {
                rb2d.set_linvel(Vec2::new(player.move_speed, 0.0).into(), true);
            } else {
                rb2d.set_linvel(Vec2::ZERO.into(), false);
            }

            if transform.position.x > 9.0 { transform.position.x = 9.0 }
            if transform.position.x < -9.0 { transform.position.x = -9.0 }
        }
    }
}

#[derive(Component, Default)]
struct Ball {
    start_velocity: Vec2,
    started: bool,
}

impl Edit for Ball {
    fn name() -> &'static str { "Ball" }

    fn get_data(&self) -> Data {
        Data::new().insert("start_velocity", Value::Vec2(self.start_velocity))
            .insert_with_limit("started", Value::String(format!("{}", self.started)), Limit::ReadOnly)
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Vec2(v)) = data.get("start_velocity") { self.start_velocity = *v }
    }
}

fn push_ball_system(mut ball: ViewMut<Ball>, rb2d: View<RigidBody2D>, mut physics2d_manager: UniqueViewMut<Physics2DManager>) {
    for (mut ball, rb2d) in (&mut ball, &rb2d).iter() {
        if !ball.started {
            if let Some(rb2d) = physics2d_manager.rigid_body_set.get_mut(rb2d.handle()) {
                rb2d.set_linvel(ball.start_velocity.into(), true);
                ball.started = true;
            }
        }
    }
}
