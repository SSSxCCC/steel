use glam::Vec2;
use shipyard::{AddComponent, Component, EntityId, IntoIter, IntoWithId, UniqueView, UniqueViewMut, View, ViewMut};
use steel::{data::{Data, Limit, Value}, edit::Edit, engine::{Command, DrawInfo, Engine, EngineImpl, FrameInfo, FrameStage, InitInfo}, input::Input, physics2d::{Collider2D, Physics2DManager, RigidBody2D}, platform::BuildTarget, scene::SceneManager, time::Time, transform::Transform, ui::EguiContext};
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

#[derive(Component, Edit, Default)]
struct Player {
    move_speed: f32,
}

fn player_control_system(player: View<Player>, mut transform: ViewMut<Transform>, rb2d: View<RigidBody2D>,
        mut physics2d_manager: UniqueViewMut<Physics2DManager>, input: UniqueView<Input>, egui_ctx: UniqueView<EguiContext>) {
    for (player, mut transform, rb2d) in (&player, &mut transform, &rb2d).iter() {
        if let Some(rb2d) = physics2d_manager.rigid_body_set.get_mut(rb2d.handle()) {
            let mut linvel = Vec2::ZERO;
            if input.key_held(VirtualKeyCode::Left) {
                linvel = Vec2::new(-player.move_speed, 0.0);
            } else if input.key_held(VirtualKeyCode::Right) {
                linvel = Vec2::new(player.move_speed, 0.0);
            }
            if steel::platform::BUILD_TARGET == BuildTarget::Android {
                egui_ctx.input(|input| {
                    if let Some(press_origin) = input.pointer.press_origin() {
                        if press_origin.x < input.screen_rect.center().x {
                            linvel = Vec2::new(-player.move_speed, 0.0);
                        } else {
                            linvel = Vec2::new(player.move_speed, 0.0);
                        }
                    }
                });
            }
            rb2d.set_linvel(linvel.into(), true);

            if transform.position.x > 9.0 { transform.position.x = 9.0 }
            if transform.position.x < -9.0 { transform.position.x = -9.0 }
        }
    }
}

#[derive(Component, Edit, Default)]
struct Ball {
    start_velocity: Vec2,
    #[edit(limit = "Limit::ReadOnly")]
    started: bool,
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

#[derive(Edit, Component, Default)]
struct Border;

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

#[derive(Edit, Component, Default)]
struct MainMenu;

fn main_menu_system(main_menu_component: View<MainMenu>, egui_ctx: UniqueView<EguiContext>, mut scene_manager: UniqueViewMut<SceneManager>) {
    for _ in main_menu_component.iter() {
        egui::CentralPanel::default().show(&egui_ctx, |ui| {
            let available_size = ui.available_size();
            let button_center = egui::pos2(available_size.x / 2.0, available_size.y / 2.0);
            let button_size = egui::vec2(200.0, 100.0);
            let button_rect = egui::Rect::from_center_size(button_center, button_size);
            if ui.put(button_rect, egui::Button::new(egui::RichText::new("Start Game").size(30.0))).clicked() {
                scene_manager.switch_scene("game.scene".into());
            }
        });
    }
}
