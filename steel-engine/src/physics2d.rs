use glam::{Quat, Vec2, Vec3, Vec3Swizzles, Vec4};
use shipyard::{AddComponent, Component, EntityId, Get, IntoIter, IntoWithId, Unique, UniqueViewMut, ViewMut};
use rapier2d::prelude::*;
use rayon::iter::ParallelIterator;
use steel_common::data::{Data, Limit, Value};
use crate::{render::canvas::Canvas, shape::ShapeWrapper, transform::Transform, edit::Edit};

#[derive(Component, Debug)]
#[track(All)]
pub struct RigidBody2D {
    handle: RigidBodyHandle,
    pub body_type: RigidBodyType,
    last_transform: Option<(Vec2, f32)>, // translation and rotation
}

impl RigidBody2D {
    pub fn new(body_type: RigidBodyType) -> Self {
        RigidBody2D { handle: RigidBodyHandle::invalid(), body_type, last_transform: None }
    }

    pub fn handle(&self) -> RigidBodyHandle {
        self.handle
    }

    pub fn i32_to_rigid_body_type(i: &i32) -> RigidBodyType {
        match i {
            0 => RigidBodyType::Dynamic,
            1 => RigidBodyType::Fixed,
            2 => RigidBodyType::KinematicPositionBased,
            3 => RigidBodyType::KinematicVelocityBased,
            _ => RigidBodyType::Dynamic,
        }
    }

    /// update last_transform and returns true if changed
    fn update_last_transform(&mut self, transform: &Transform) -> bool {
        let rotation = transform.rotation.to_scaled_axis().z;
        if let Some(last_transform) = self.last_transform {
            if last_transform.0 == transform.position.xy() && last_transform.1 == rotation {
                return false;
            }
        }
        self.last_transform = Some((transform.position.xy(), rotation));
        true
    }
}

impl Default for RigidBody2D {
    fn default() -> Self {
        Self { handle: RigidBodyHandle::invalid(), body_type: RigidBodyType::Dynamic, last_transform: None }
    }
}

impl Edit for RigidBody2D {
    fn name() -> &'static str { "RigidBody2D" }

    fn get_data(&self) -> Data {
        let mut data = Data::new();
        data.add("handle", Value::String(format!("{:?}", self.handle)), Limit::ReadOnly);
        data.add("body_type", Value::Int32(self.body_type as i32),
            Limit::Int32Enum(vec![(0, "Dynamic".into()), (1, "Fixed".into()),
            (2, "KinematicPositionBased".into()), (3, "KinematicVelocityBased".into())]));
        data
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Int32(i)) = data.values.get("body_type") { self.body_type = Self::i32_to_rigid_body_type(i) }
    }
}

#[derive(Component, Debug)]
#[track(All)]
pub struct Collider2D {
    handle: ColliderHandle,
    pub shape: ShapeWrapper,
    pub restitution: f32,
    last_transform: Option<(Vec2, f32, Vec2)>, // translation, rotation and scale
}

impl Collider2D {
    pub fn new(shape: SharedShape, restitution: f32) -> Self {
        Collider2D { handle: ColliderHandle::invalid(), shape: ShapeWrapper(shape), restitution, last_transform: None }
    }

    pub fn handle(&self) -> ColliderHandle {
        self.handle
    }

    /// update last_transform and returns true if changed
    fn update_last_transform(&mut self, transform: &Transform) -> bool {
        let rotation = transform.rotation.to_scaled_axis().z;
        if let Some(last_transform) = self.last_transform {
            if last_transform.0 == transform.position.xy() && last_transform.1 == rotation && last_transform.2 == transform.scale.xy() {
                return false;
            }
        }
        self.last_transform = Some((transform.position.xy(), rotation, transform.scale.xy()));
        true
    }

    fn scaled_shape(&self, transform: &Transform) -> SharedShape {
        let scale = transform.scale.xy();
        match self.shape.shape_type() {
            ShapeType::Ball => {
                let scale = std::cmp::max_by(scale.x.abs(), scale.y.abs(), |x, y| x.partial_cmp(y).unwrap());
                let radius = self.shape.as_ball().unwrap().radius * scale;
                SharedShape::new(Ball::new(radius))
            },
            ShapeType::Cuboid => {
                SharedShape::new(self.shape.as_cuboid().unwrap().scaled(&vector![scale.x, scale.y]))
            }
            _ => self.shape.clone(), // TODO: support all shape type
        }
    }
}

impl Default for Collider2D {
    fn default() -> Self {
        Self { handle: ColliderHandle::invalid(), shape: ShapeWrapper(SharedShape::cuboid(0.5, 0.5)), restitution: Default::default(), last_transform: None }
    }
}

impl Edit for Collider2D {
    fn name() -> &'static str { "Collider2D" }

    fn get_data(&self) -> Data {
        let mut data = Data::new();
        data.add("handle", Value::String(format!("{:?}", self.handle)), Limit::ReadOnly);
        self.shape.get_data(&mut data);
        data.values.insert("restitution".into(), Value::Float32(self.restitution));
        data
    }

    fn set_data(&mut self, data: &Data) {
        self.shape.set_data(data);
        if let Some(Value::Float32(f)) = data.values.get("restitution") { self.restitution = *f };
    }
}

#[derive(Unique)]
pub struct Physics2DManager {
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub gravity: Vector<Real>,
    pub integration_parameters: IntegrationParameters,
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: BroadPhase,
    pub narrow_phase: NarrowPhase,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub physics_hooks: Box<dyn PhysicsHooks>,
    pub event_handler: Box<dyn EventHandler>,
    pub debug_render_pipeline: DebugRenderPipeline,
}

impl Physics2DManager {
    pub fn update(&mut self) {
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            None,
            self.physics_hooks.as_ref(),
            self.event_handler.as_ref(),
        );
    }
}

impl Default for Physics2DManager {
    fn default() -> Self {
        Physics2DManager { rigid_body_set: RigidBodySet::new(), collider_set: ColliderSet::new(), gravity: vector![0.0, -9.81],
            integration_parameters: IntegrationParameters::default(), physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(), broad_phase: BroadPhase::new(), narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(), multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(), physics_hooks: Box::new(()), event_handler: Box::new(()),
            debug_render_pipeline: DebugRenderPipeline::default() }
    }
}

impl Edit for Physics2DManager {
    fn name() -> &'static str { "Physics2DManager" }

    fn get_data(&self) -> Data {
        let mut data = Data::new();
        data.values.insert("gravity".into(), Value::Vec2(Vec2::new(self.gravity.x, self.gravity.y)));
        data
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Vec2(v)) = data.values.get("gravity") { self.gravity = vector![v.x, v.y] }
    }
}

pub fn physics2d_maintain_system(mut physics2d_manager: UniqueViewMut<Physics2DManager>,
        mut rb2d: ViewMut<RigidBody2D>, mut col2d: ViewMut<Collider2D>,
        mut transform: ViewMut<Transform>) {
    let physics2d_manager = physics2d_manager.as_mut();

    for e in rb2d.removed() {
        log::warn!("Leak warning: RigidBody2D component of entity({e:?}) has been removed, \
            we don't know its handle so that its body can not be removed from physics world! \
            please use delete instead of remove on RigidBody2D component");
    }

    for e in col2d.removed() {
        log::warn!("Leak warning: Collider2D component of entity({e:?}) has been removed, \
            we don't know its handle so that its body can not be removed from physics world! \
            please use delete instead of remove on Collider2D component");
    }

    for (_, rb2d) in rb2d.deleted() {
        if physics2d_manager.rigid_body_set.contains(rb2d.handle) {
            physics2d_manager.rigid_body_set.remove(rb2d.handle,
                &mut physics2d_manager.island_manager, &mut physics2d_manager.collider_set,
                &mut physics2d_manager.impulse_joint_set, &mut physics2d_manager.multibody_joint_set,
                false);
        }
    }

    for (_, col2d) in col2d.deleted() {
        if physics2d_manager.collider_set.contains(col2d.handle) {
            physics2d_manager.collider_set.remove(col2d.handle,
                &mut physics2d_manager.island_manager, &mut physics2d_manager.rigid_body_set,
                true);
        }
    }

    for (e, mut rb2d) in rb2d.inserted_or_modified_mut().iter().with_id() {
        if let Some(rigid_body) = physics2d_manager.rigid_body_set.get_mut(rb2d.handle) {
            rigid_body.set_body_type(rb2d.body_type, true);
        } else {
            if !transform.contains(e) {
                transform.add_component_unchecked(e, Transform::default());
            }
            let transform = transform.get(e).unwrap();
            let rigid_body = RigidBodyBuilder::new(rb2d.body_type)
                    .translation(vector![transform.position.x, transform.position.y])
                    .rotation(transform.rotation.to_scaled_axis().z).build();
            rb2d.handle = physics2d_manager.rigid_body_set.insert(rigid_body);
        }

        if let Ok(col2d) = col2d.get(e) {
            if physics2d_manager.collider_set.contains(col2d.handle) {
                physics2d_manager.collider_set.set_parent(col2d.handle, Some(rb2d.handle), &mut physics2d_manager.rigid_body_set)
            }
        }
    }

    for (e, mut col2d) in col2d.inserted_or_modified_mut().iter().with_id() {
        if !transform.contains(e) {
            transform.add_component_unchecked(e, Transform::default());
        }
        let transform = transform.get(e).unwrap();
        let shape = col2d.scaled_shape(transform);

        if let Some(collider) = physics2d_manager.collider_set.get_mut(col2d.handle) {
            collider.set_shape(shape);
            collider.set_restitution(col2d.restitution);
        } else {
            let mut collider = ColliderBuilder::new(shape).restitution(col2d.restitution).build();
            if let Ok(rb2d) = &rb2d.get(e) {
                // TODO: add position and rotation relative to parent
                col2d.handle = physics2d_manager.collider_set.insert_with_parent(collider, rb2d.handle, &mut physics2d_manager.rigid_body_set);
            } else {
                collider.set_translation(vector![transform.position.x, transform.position.y]);
                collider.set_rotation(Rotation::from_angle(transform.rotation.to_scaled_axis().z));
                col2d.handle = physics2d_manager.collider_set.insert(collider);
            }
        }
    }

    // also call this in maintain system for steel-editor when update systems do not run
    // TODO: do not call this when update systems are running
    physics2d_update_from_transform(physics2d_manager, &transform, &mut rb2d, &mut col2d);

    rb2d.clear_all_removed_and_deleted();
    col2d.clear_all_removed_and_deleted();
    rb2d.clear_all_inserted_and_modified();
    col2d.clear_all_inserted_and_modified();
}

fn physics2d_update_from_transform(physics2d_manager: &mut Physics2DManager, transform: &ViewMut<Transform>,
        rb2d: &mut ViewMut<RigidBody2D>, col2d: &mut ViewMut<Collider2D>) {
    // TODO: find a way to use par_iter here (&mut physics2d_manager.rigid_body_set can not be passed to par_iter)
    for (transform, mut rb2d) in (transform, rb2d).iter() {
        if rb2d.update_last_transform(transform) {
            if let Some(rb2d) = physics2d_manager.rigid_body_set.get_mut(rb2d.handle) {
                rb2d.set_translation(vector![transform.position.x, transform.position.y], true);
                rb2d.set_rotation(Rotation::from_angle(transform.rotation.to_scaled_axis().z), true);
            }
        }
    }

    for (transform, mut col2d) in (transform, col2d).iter() {
        if col2d.update_last_transform(transform) {
            if let Some(collider) = physics2d_manager.collider_set.get_mut(col2d.handle) {
                collider.set_shape(col2d.scaled_shape(transform));
                collider.set_translation(vector![transform.position.x, transform.position.y]);
                collider.set_rotation(Rotation::from_angle(transform.rotation.to_scaled_axis().z));
            }
        }
    }
}

pub fn physics2d_update_system(mut physics2d_manager: UniqueViewMut<Physics2DManager>,
        mut rb2d: ViewMut<RigidBody2D>, mut col2d: ViewMut<Collider2D>, mut transform: ViewMut<Transform>) {
    let physics2d_manager = physics2d_manager.as_mut();
    physics2d_update_from_transform(physics2d_manager, &transform, &mut rb2d, &mut col2d);

    physics2d_manager.update();

    (&mut rb2d, &mut transform).par_iter().for_each(|(mut rb2d, transform)| {
        let rigid_body = &physics2d_manager.rigid_body_set[rb2d.handle];
        transform.position.x = rigid_body.translation().x;
        transform.position.y = rigid_body.translation().y;
        let mut rotation = transform.rotation.to_scaled_axis();
        rotation.z = rigid_body.rotation().angle();
        transform.rotation = Quat::from_scaled_axis(rotation);
        rb2d.update_last_transform(transform);
    });

    (&mut col2d, &transform).par_iter().for_each(|(mut col2d, transform)| {
        col2d.update_last_transform(transform);
    });
}

struct DebugRenderer<'a> {
    canvas: &'a mut Canvas
}

impl DebugRenderBackend for DebugRenderer<'_> {
    fn draw_line(&mut self, _: DebugRenderObject, a: Point<Real>, b: Point<Real>, color: [f32; 4]) {
        // currently we use a big z value to make sure that debug render content can be seen
        // TODO: find a better way to make sure the visiblity of debug render content
        let color = Vec4::from_array(color);
        self.canvas.line(Vec3::new(a.x, a.y, 1000.0), Vec3::new(b.x, b.y, 1000.0), color, EntityId::dead()); // TODO: eid
    }
}

pub fn physics2d_debug_render_system(mut physics2d_manager: UniqueViewMut<Physics2DManager>, mut canvas: UniqueViewMut<Canvas>) {
    let physics2d_manager = physics2d_manager.as_mut();
    let mut debug_render_backend = DebugRenderer { canvas: &mut canvas };
    physics2d_manager.debug_render_pipeline.render(
        &mut debug_render_backend,
        &physics2d_manager.rigid_body_set,
        &physics2d_manager.collider_set,
        &physics2d_manager.impulse_joint_set,
        &physics2d_manager.multibody_joint_set,
        &physics2d_manager.narrow_phase
    );
}
