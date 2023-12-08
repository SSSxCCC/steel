pub use steel_common;
use steel_common::{
    shipyard::{Unique, UniqueViewMut, ViewMut, View, World, AddComponent, Get},
    ComponentData,
    WorldData,
    EntityData,
    glam::{Vec3, Vec2},
    rapier2d::{dynamics::{RigidBodySet, IntegrationParameters, IslandManager, ImpulseJointSet, MultibodyJointSet, CCDSolver, RigidBodyBuilder, RigidBodyHandle, RigidBodyType},
    geometry::{ColliderSet, BroadPhase, NarrowPhase, ColliderBuilder, SharedShape, ColliderHandle}, math::{Real, Vector}, pipeline::{PhysicsPipeline, PhysicsHooks, EventHandler}},
    Variant,
    Value, rayon::iter::ParallelIterator};
use shipyard::{Component, IntoIter, IntoWithId};
use rapier2d::prelude::*;

#[derive(Component, Debug)]
struct Renderer2D; // can only render cuboid currently. TODO: render multiple shape

#[derive(Unique)]
struct Physics2DManager {
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
    fn new() -> Self {
        Physics2DManager { rigid_body_set: RigidBodySet::new(), collider_set: ColliderSet::new(), gravity: vector![0.0, -9.81],
            integration_parameters: IntegrationParameters::default(), physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(), broad_phase: BroadPhase::new(), narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(), multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(), physics_hooks: Box::new(()), event_handler: Box::new(()) }
    }

    fn update(&mut self) {
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

fn physics2d_maintain_system(mut physics2d_manager: UniqueViewMut<Physics2DManager>,
        mut rb2d: ViewMut<RigidBody2D>, mut col2d: ViewMut<Collider2D>,
        mut transform2d: ViewMut<Transform2D>) {
    let physics2d_manager = physics2d_manager.as_mut();
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

    rb2d.clear_all_inserted_and_modified();
    col2d.clear_all_inserted_and_modified();
}

fn physics2d_update_system(mut physics2d_manager: UniqueViewMut<Physics2DManager>,
        rb2d: View<RigidBody2D>, mut transform2d: ViewMut<Transform2D>) {
    physics2d_manager.update();
    (&rb2d, &mut transform2d).par_iter().for_each(|(rb2d, mut transform2d)| {
        let rigid_body = &physics2d_manager.rigid_body_set[rb2d.handle];
        transform2d.position.x = rigid_body.translation().x;
        transform2d.position.y = rigid_body.translation().y;
        transform2d.rotation = rigid_body.rotation().angle();
    });
}

trait Edit: Component {
    fn name() -> &'static str;

    fn to_data(&self) -> ComponentData {
        ComponentData::new(Self::name())
    }

    fn from_data(&mut self, data: ComponentData) { }
}

fn add_component<T: Edit + Send + Sync>(world_data: &mut WorldData, world: &World) {
    world.run(|c: View<T>| {
        for (e, c) in c.iter().with_id() {
            let index = *world_data.id_index_map.entry(e).or_insert(world_data.entities.len());
            if index == world_data.entities.len() {
                world_data.entities.push(EntityData { id: e, components: Vec::new() });
            }
            world_data.entities[index].components.push(c.to_data());
        }
    })
}


#[derive(Component, Debug, Default)]
struct Transform2D {
    position: Vec3,
    rotation: f32, // radian
    scale: Vec2
}

impl Edit for Transform2D {
    fn name() -> &'static str { "Transform2D" }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::new(Self::name());
        data.variants.push(Variant { name: "position", value: Value::Vec3(self.position) });
        data.variants.push(Variant { name: "rotation", value: Value::Float32(self.rotation) });
        data.variants.push(Variant { name: "scale", value: Value::Vec2(self.scale) });
        data
    }

    fn from_data(&mut self, data: ComponentData) {
        for v in data.variants {
            match v.name {
                "position" => self.position = if let Value::Vec3(position) = v.value { position } else { Default::default() },
                "rotation" => self.rotation = if let Value::Float32(rotation) = v.value { rotation } else { Default::default() },
                "scale" => self.scale = if let Value::Vec2(scale) = v.value { scale } else { Vec2::ONE },
                _ => (),
            }
        }
    }
}

#[derive(Component, Debug)]
#[track(All)]
struct RigidBody2D {
    handle: RigidBodyHandle,
    body_type: RigidBodyType,
}

impl RigidBody2D {
    fn new(body_type: RigidBodyType) -> Self {
        RigidBody2D { handle: RigidBodyHandle::invalid(), body_type }
    }
}

impl Edit for RigidBody2D {
    fn name() -> &'static str { "RigidBody2D" }
}

struct ShapeWrapper(SharedShape);

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
struct Collider2D {
    handle: ColliderHandle,
    shape: ShapeWrapper,
    restitution: f32,
}

impl Collider2D {
    fn new(shape: SharedShape, restitution: f32) -> Self {
        Collider2D { handle: ColliderHandle::invalid(), shape: ShapeWrapper(shape), restitution }
    }
}

impl Edit for Collider2D {
    fn name() -> &'static str { "Collider2D" }
}
