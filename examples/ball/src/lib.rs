use glam::Vec2;
use shipyard::{
    AddComponent, Component, EntityId, IntoIter, IntoWithId, UniqueView, UniqueViewMut, View,
    ViewMut,
};
use steel::{
    app::{App, Schedule, SteelApp},
    asset::AssetManager,
    data::{Data, Limit, Value},
    edit::Edit,
    input::Input,
    physics2d::{
        Collider2D, Physics2DManager, Physics2DPlugin, RigidBody2D, PHYSICS2D_UPDATE_SYSTEM_ORDER,
    },
    platform::BuildTarget,
    scene::SceneManager,
    time::Time,
    transform::Transform,
    ui::EguiContext,
};
use winit::event::VirtualKeyCode;

#[no_mangle]
pub fn create() -> Box<dyn App> {
    SteelApp::new()
        .add_plugin(Physics2DPlugin)
        .register_component::<Player>()
        .register_component::<Ball>()
        .register_component::<Border>()
        .register_component::<MainMenu>()
        .add_system(Schedule::PreUpdate, 0, main_menu_system)
        .add_system(Schedule::Update, 0, player_control_system)
        .add_system(Schedule::Update, 0, push_ball_system)
        .add_system(
            Schedule::Update,
            PHYSICS2D_UPDATE_SYSTEM_ORDER + 1,
            border_check_system,
        )
        .add_system(Schedule::Update, 0, lose_system)
        .boxed()
}

#[derive(Component, Edit, Default)]
struct Player {
    move_speed: f32,
}

fn player_control_system(
    player: View<Player>,
    mut transform: ViewMut<Transform>,
    rb2d: View<RigidBody2D>,
    mut physics2d_manager: UniqueViewMut<Physics2DManager>,
    input: UniqueView<Input>,
    egui_ctx: UniqueView<EguiContext>,
) {
    for (player, transform, rb2d) in (&player, &mut transform, &rb2d).iter() {
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

            if transform.position.x > 9.0 {
                transform.position.x = 9.0
            }
            if transform.position.x < -9.0 {
                transform.position.x = -9.0
            }
        }
    }
}

#[derive(Component, Edit, Default)]
struct Ball {
    start_velocity: Vec2,
    #[edit(limit = "Limit::ReadOnly")]
    started: bool,
}

fn push_ball_system(
    mut ball: ViewMut<Ball>,
    rb2d: View<RigidBody2D>,
    mut physics2d_manager: UniqueViewMut<Physics2DManager>,
) {
    for (ball, rb2d) in (&mut ball, &rb2d).iter() {
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

fn border_check_system(
    border: View<Border>,
    ball: View<Ball>,
    mut lose: ViewMut<Lose>,
    col2d: View<Collider2D>,
    physics2d_manager: UniqueView<Physics2DManager>,
) {
    let mut border_entity = EntityId::dead();
    for (entity, (_border, border_col2d, _)) in (&border, &col2d, !&lose).iter().with_id() {
        for (_ball, ball_col2d) in (&ball, &col2d).iter() {
            let intersection_pair = physics2d_manager
                .narrow_phase
                .intersection_pair(border_col2d.handle(), ball_col2d.handle());
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

fn lose_system(
    mut lose: ViewMut<Lose>,
    time: UniqueView<Time>,
    egui_ctx: UniqueView<EguiContext>,
    mut scene_manager: UniqueViewMut<SceneManager>,
    asset_manager: UniqueView<AssetManager>,
) {
    for lose in (&mut lose).iter() {
        egui::CentralPanel::default().show(&egui_ctx, |ui| {
            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |ui| {
                    ui.label(egui::RichText::new("You lose!").size(100.0));
                },
            );
        });

        lose.lose_time -= time.delta();
        if lose.lose_time < 0.0 {
            if let Some(main_scene) = asset_manager.get_asset_id("main.scene") {
                scene_manager.switch_scene(main_scene);
            }
        }
    }
}

#[derive(Edit, Component, Default)]
struct MainMenu;

fn main_menu_system(
    main_menu_component: View<MainMenu>,
    egui_ctx: UniqueView<EguiContext>,
    mut scene_manager: UniqueViewMut<SceneManager>,
    asset_manager: UniqueView<AssetManager>,
) {
    for _ in main_menu_component.iter() {
        egui::CentralPanel::default().show(&egui_ctx, |ui| {
            let available_size = ui.available_size();
            let button_center = egui::pos2(available_size.x / 2.0, available_size.y / 2.0);
            let button_size = egui::vec2(200.0, 100.0);
            let button_rect = egui::Rect::from_center_size(button_center, button_size);
            if ui
                .put(
                    button_rect,
                    egui::Button::new(egui::RichText::new("Start Game").size(30.0)),
                )
                .clicked()
            {
                if let Some(game_scene) = asset_manager.get_asset_id("game.scene") {
                    scene_manager.switch_scene(game_scene);
                }
            }
        });
    }
}
