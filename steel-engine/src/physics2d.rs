use glam::Vec2;
use indexmap::IndexMap;
use shipyard::{Component, IntoIter, IntoWithId, Unique, UniqueViewMut, ViewMut, View, AddComponent, Get};
use rapier2d::prelude::*;
use rayon::iter::ParallelIterator;
use steel_common::{ComponentData, Limit, Value};
use crate::{Transform2D, Edit};

#[derive(Component, Debug)]
#[track(All)]
pub struct RigidBody2D {
    handle: RigidBodyHandle,
    body_type: RigidBodyType,
}

impl RigidBody2D {
    pub fn new(body_type: RigidBodyType) -> Self {
        RigidBody2D { handle: RigidBodyHandle::invalid(), body_type }
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
}

impl Default for RigidBody2D {
    fn default() -> Self {
        Self { handle: RigidBodyHandle::invalid(), body_type: RigidBodyType::Dynamic }
    }
}

impl Edit for RigidBody2D {
    fn name() -> &'static str { "RigidBody2D" }

    fn get_data(&self) -> ComponentData {
        let mut data = ComponentData::new();
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
}

impl Collider2D {
    pub fn new(shape: SharedShape, restitution: f32) -> Self {
        Collider2D { handle: ColliderHandle::invalid(), shape: ShapeWrapper(shape), restitution }
    }

    pub fn i32_to_shape_type(i: &i32) -> ShapeType {
        match i {
            0 => ShapeType::Ball,
            1 => ShapeType::Cuboid,
            2 => ShapeType::Capsule,
            3 => ShapeType::Segment,
            4 => ShapeType::Triangle,
            _ => ShapeType::Ball, // TODO: support all shape type
        }
    }

    fn get_shape_data(&self, data: &mut ComponentData) {
        data.add("shape_type", Value::Int32(self.shape.shape_type() as i32),
            Limit::Int32Enum(vec![(0, "Ball".into()), (1, "Cuboid".into()), (2, "Capsule".into()), (3, "Segment".into()), (4, "Triangle".into())]));
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
}

impl Default for Collider2D {
    fn default() -> Self {
        Self { handle: ColliderHandle::invalid(), shape: ShapeWrapper(SharedShape::cuboid(0.5, 0.5)), restitution: Default::default() }
    }
}

impl Edit for Collider2D {
    fn name() -> &'static str { "Collider2D" }

    fn get_data(&self) -> ComponentData {
        let mut data = ComponentData::new();
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
}

impl Physics2DManager {
    pub fn new() -> Self {
        Physics2DManager { rigid_body_set: RigidBodySet::new(), collider_set: ColliderSet::new(), gravity: vector![0.0, -9.81],
            integration_parameters: IntegrationParameters::default(), physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(), broad_phase: BroadPhase::new(), narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(), multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(), physics_hooks: Box::new(()), event_handler: Box::new(()) }
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
        mut transform2d: ViewMut<Transform2D>) {
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
            if !transform2d.contains(e) {
                transform2d.add_component_unchecked(e, Transform2D::default());
            }
            let transform2d = transform2d.get(e).unwrap();
            let rigid_body = RigidBodyBuilder::new(rb2d.body_type)
                    .translation(vector![transform2d.position.x, transform2d.position.y])
                    .rotation(transform2d.rotation).build();
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
            if !transform2d.contains(e) {
                transform2d.add_component_unchecked(e, Transform2D::default());
            }
            let transform2d = transform2d.get(e).unwrap();
            let mut collider = ColliderBuilder::new(col2d.shape.clone()).restitution(col2d.restitution).build();
            if let Ok(rb2d) = &rb2d.get(e) {
                // TODO: add position and rotation relative to parent
                col2d.handle = physics2d_manager.collider_set.insert_with_parent(collider, rb2d.handle, &mut physics2d_manager.rigid_body_set);
            } else {
                collider.set_translation(vector![transform2d.position.x, transform2d.position.y]);
                //collider.set_rotation(transform2d.rotation); TODO: how to set_rotation?
                col2d.handle = physics2d_manager.collider_set.insert(collider);
            }
        }
    }

    rb2d.clear_all_removed_and_deleted();
    col2d.clear_all_removed_and_deleted();
    rb2d.clear_all_inserted_and_modified();
    col2d.clear_all_inserted_and_modified();
}

pub fn physics2d_update_system(mut physics2d_manager: UniqueViewMut<Physics2DManager>,
        rb2d: View<RigidBody2D>, mut transform2d: ViewMut<Transform2D>) {
    physics2d_manager.update();
    (&rb2d, &mut transform2d).par_iter().for_each(|(rb2d, mut transform2d)| {
        let rigid_body = &physics2d_manager.rigid_body_set[rb2d.handle];
        transform2d.position.x = rigid_body.translation().x;
        transform2d.position.y = rigid_body.translation().y;
        transform2d.rotation = rigid_body.rotation().angle();
    });
}