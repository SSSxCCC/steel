use glam::Vec2;
use shipyard::{AddComponent, Component, EntityId, IntoIter, IntoWithId, UniqueView, UniqueViewMut, View, ViewMut};
use steel::{data::{Data, Limit, Value}, edit::Edit, engine::{Command, DrawInfo, Engine, EngineImpl, FrameInfo, FrameStage, InitInfo}, input::Input, physics2d::{Collider2D, Physics2DManager, RigidBody2D}, scene::SceneManager, time::Time, transform::Transform, ui::EguiContext};
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
        self.inner.register_component::<Border>();
        self.inner.register_component::<MainMenu>();
    }

    fn frame(&mut self, info: &FrameInfo) {
        self.inner.frame(info);
        match info.stage {
            FrameStage::Maintain => {
                self.inner.world.run(main_menu_system);
            },
            FrameStage::Update => {
                self.inner.world.run(player_control_system);
                self.inner.world.run(push_ball_system);
                self.inner.world.run(border_check_system);
                self.inner.world.run(lose_system);
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

#[derive(Component, Default)]
struct Border;

impl Edit for Border {
    fn name() -> &'static str { "Border" }
}

fn border_check_system(border: View<Border>, ball: View<Ball>, mut lose: ViewMut<Lose>, col2d: View<Collider2D>, physics2d_manager: UniqueView<Physics2DManager>) {
    let mut border_entity = EntityId::dead();
    for (entity, (_border, border_col2d, _)) in (&border, &col2d, !&lose).iter().with_id() {
        for (_ball, ball_col2d) in (&ball, &col2d).iter() {
            let intersection_pair = physics2d_manager.narrow_phase.intersection_pair(border_col2d.handle(), ball_col2d.handle());
            if intersection_pair.is_none() {
                border_entity = entity;
            }
        }
    }
    if border_entity != EntityId::dead() {
        lose.add_component_unchecked(border_entity, Lose::default());
    }
}

#[derive(Component)]
struct Lose {
    lose_time: f32,
}

impl Default for Lose {
    fn default() -> Self {
        Lose { lose_time: 5.0 }
    }
}

fn lose_system(mut lose: ViewMut<Lose>, time: UniqueView<Time>, egui_ctx: UniqueView<EguiContext>, mut scene_manager: UniqueViewMut<SceneManager>) {
    for lose in (&mut lose).iter() {
        egui::CentralPanel::default().show(&egui_ctx, |ui| {
            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                ui.label(egui::RichText::new("You lose!").size(100.0));
            });
        });

        lose.lose_time -= time.delta();
        if lose.lose_time < 0.0 {
            scene_manager.switch_scene("main.scene".into());
        }
    }
}

#[derive(Component, Default)]
struct MainMenu;

impl Edit for MainMenu {
    fn name() -> &'static str { "MainMenu" }
}

fn main_menu_system(main_menu_component: View<MainMenu>, egui_ctx: UniqueView<EguiContext>, mut scene_manager: UniqueViewMut<SceneManager>) {
    for _ in main_menu_component.iter() {
        egui::CentralPanel::default().show(&egui_ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                if ui.button(egui::RichText::new("Start Game").size(30.0)).clicked() {
                    scene_manager.switch_scene("game.scene".into());
                }
            });
        });
    }
}
