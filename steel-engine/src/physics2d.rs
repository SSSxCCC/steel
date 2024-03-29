use glam::{Quat, Vec2, Vec3, Vec3Swizzles, Vec4};
use indexmap::IndexMap;
use shipyard::{Component, IntoIter, IntoWithId, Unique, UniqueViewMut, ViewMut, AddComponent, Get};
use rapier2d::prelude::*;
use rayon::iter::ParallelIterator;
use steel_common::{ComponentData, Limit, Value};
use crate::{render::canvas::Canvas, Edit, transform::Transform};

#[derive(Component, Debug)]
#[track(All)]
pub struct RigidBody2D {
    handle: RigidBodyHandle,
    body_type: RigidBodyType,
    last_transform: Option<(Vec2, f32)>, // translation and rotation
}

impl RigidBody2D {
    pub fn new(body_type: RigidBodyType) -> Self {
        RigidBody2D { handle: RigidBodyHandle::invalid(), body_type, last_transform: None }
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

    fn get_data(&self) -> ComponentData {
        let mut data = ComponentData::new();
        data.add("handle", Value::String(format!("{:?}", self.handle)), Limit::ReadOnly);
        data.add("body_type", Value::Int32(self.body_type as i32),
            Limit::Int32Enum(vec![(0, "Dynamic".into()), (1, "Fixed".into()),
            (2, "KinematicPositionBased".into()), (3, "KinematicVelocityBased".into())]));
        data
    }

    fn set_data(&mut self, data: &ComponentData) {
        if let Some(Value::Int32(i)) = data.values.get("body_type") { self.body_type = Self::i32_to_rigid_body_type(i) }
    }
}

pub struct ShapeWrapper(SharedShape);

impl std::ops::Deref for ShapeWrapper {
    type Target = SharedShape;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Debug for ShapeWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ShapeWrapper").field(&self.shape_type()).finish() // TODO: print all members
    }
}

#[derive(Component, Debug)]
#[track(All)]
pub struct Collider2D {
    handle: ColliderHandle,
    shape: ShapeWrapper,
    restitution: f32,
    last_transform: Option<(Vec2, f32, Vec2)>, // translation, rotation and scale
}

impl Collider2D {
    pub fn new(shape: SharedShape, restitution: f32) -> Self {
        Collider2D { handle: ColliderHandle::invalid(), shape: ShapeWrapper(shape), restitution, last_transform: None }
    }

    pub fn i32_to_shape_type(i: &i32) -> ShapeType {
        match i {
            0 => ShapeType::Ball,
            1 => ShapeType::Cuboid,
            _ => ShapeType::Ball, // TODO: support all shape type
        }
    }

    fn get_shape_data(&self, data: &mut ComponentData) {
        data.add("shape_type", Value::Int32(self.shape.shape_type() as i32),
            Limit::Int32Enum(vec![(0, "Ball".into()), (1, "Cuboid".into())]));
        if let Some(shape) = self.shape.as_ball() {
            data.values.insert("radius".into(), Value::Float32(shape.radius));
        } else if let Some(shape) = self.shape.as_cuboid() {
            data.values.insert("size".into(), Value::Vec2(Vec2::new(shape.half_extents.x * 2.0, shape.half_extents.y * 2.0)));
        } // TODO: support all shape type
    }

    fn set_shape_data(&mut self, values: &IndexMap<String, Value>) {
        if let Some(Value::Int32(i)) = values.get("shape_type") {
            let shape_type = Self::i32_to_shape_type(i);
            match shape_type {
                ShapeType::Ball => { // We have to create a new shape because SharedShape::as_shape_mut method can not compile
                    let mut shape = if let Some(shape) = self.shape.as_ball() { *shape } else { Ball::new(0.5) };
                    if let Some(Value::Float32(f)) = values.get("radius") { shape.radius = *f }
                    self.shape = ShapeWrapper(SharedShape::new(shape));
                },
                ShapeType::Cuboid => {
                    let mut shape = if let Some(shape) = self.shape.as_cuboid() { *shape } else { Cuboid::new([0.5, 0.5].into()) };
                    if let Some(Value::Vec2(v)) = values.get("size") { (shape.half_extents.x, shape.half_extents.y) = (v.x / 2.0, v.y / 2.0) }
                    self.shape = ShapeWrapper(SharedShape::new(shape));
                },
                _ => (), // TODO: support all shape type
            }
        }
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
}

impl Default for Collider2D {
    fn default() -> Self {
        Self { handle: ColliderHandle::invalid(), shape: ShapeWrapper(SharedShape::cuboid(0.5, 0.5)), restitution: Default::default(), last_transform: None }
    }
}

impl Edit for Collider2D {
    fn name() -> &'static str { "Collider2D" }

    fn get_data(&self) -> ComponentData {
        let mut data = ComponentData::new();
        data.add("handle", Value::String(format!("{:?}", self.handle)), Limit::ReadOnly);
        self.get_shape_data(&mut data);
        data.values.insert("restitution".into(), Value::Float32(self.restitution));
        data
    }

    fn set_data(&mut self, data: &ComponentData) {
        self.set_shape_data(&data.values);
        if let Some(Value::Float32(f)) = data.values.get("restitution") { self.restitution = *f };
    }
}

#[derive(Unique)]
pub struct Physics2DManager {
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    gravity: Vector<Real>,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    physics_hooks: Box<dyn PhysicsHooks>,
    event_handler: Box<dyn EventHandler>,
    debug_render_pipeline: DebugRenderPipeline,
}

impl Physics2DManager {
    pub fn new() -> Self {
        Physics2DManager { rigid_body_set: RigidBodySet::new(), collider_set: ColliderSet::new(), gravity: vector![0.0, -9.81],
            integration_parameters: IntegrationParameters::default(), physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(), broad_phase: BroadPhase::new(), narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(), multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(), physics_hooks: Box::new(()), event_handler: Box::new(()),
            debug_render_pipeline: DebugRenderPipeline::default() }
    }

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
        if let Some(collider) = physics2d_manager.collider_set.get_mut(col2d.handle) {
            collider.set_shape(col2d.shape.clone());
            collider.set_restitution(col2d.restitution);
        } else {
            if !transform.contains(e) {
                transform.add_component_unchecked(e, Transform::default());
            }
            let transform = transform.get(e).unwrap();
            let mut collider = ColliderBuilder::new(col2d.shape.clone()).restitution(col2d.restitution).build();
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
            if let Some(col2d) = physics2d_manager.collider_set.get_mut(col2d.handle) {
                col2d.set_translation(vector![transform.position.x, transform.position.y]);
                col2d.set_rotation(Rotation::from_angle(transform.rotation.to_scaled_axis().z));
                // TODO: scale
            }
        }
    }
}

pub fn physics2d_update_system(mut physics2d_manager: UniqueViewMut<Physics2DManager>,
        mut rb2d: ViewMut<RigidBody2D>, mut col2d: ViewMut<Collider2D>, mut transform: ViewMut<Transform>) {
    let physics2d_manager = physics2d_manager.as_mut();
    physics2d_update_from_transform(physics2d_manager, &transform, &mut rb2d, &mut col2d);

    physics2d_manager.update();

    (&mut rb2d, &mut transform).par_iter().for_each(|(mut rb2d, mut transform)| {
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
        self.canvas.lines.push([(Vec3::new(a.x, a.y, 1000.0), color), (Vec3::new(b.x, b.y, 1000.0), color)]);
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
