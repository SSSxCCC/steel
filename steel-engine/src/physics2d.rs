use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use shipyard::{AddComponent, Component, EntityId, Get, IntoIter, IntoWithId, Unique, UniqueView, UniqueViewMut, View, ViewMut};
use rapier2d::prelude::*;
use rayon::iter::ParallelIterator;
use steel_common::data::{Data, Limit, Value};
use crate::{edit::Edit, hierarchy::Child, render::canvas::Canvas, shape::ShapeWrapper, time::Time, transform::{Mat4Transform2D, Transform}};

pub const F32_MAX_DIFF: f32 = 0.000001;

/// A 2D rigid body in physics world of rapier2d.
#[derive(Component, Debug)]
#[track(All)]
pub struct RigidBody2D {
    handle: RigidBodyHandle,
    pub body_type: RigidBodyType,
    last_transform: Option<(Vec2, f32)>, // translation and rotation
}

impl RigidBody2D {
    /// Create a RigidBody2D with body_type.
    pub fn new(body_type: RigidBodyType) -> Self {
        RigidBody2D { handle: RigidBodyHandle::invalid(), body_type, last_transform: None }
    }

    /// Get the raw handle of this rigid body.
    pub fn handle(&self) -> RigidBodyHandle {
        self.handle
    }

    /// Convert i32 to RigidBodyType.
    pub fn i32_to_rigid_body_type(i: &i32) -> RigidBodyType {
        match i {
            0 => RigidBodyType::Dynamic,
            1 => RigidBodyType::Fixed,
            2 => RigidBodyType::KinematicPositionBased,
            3 => RigidBodyType::KinematicVelocityBased,
            _ => RigidBodyType::Dynamic,
        }
    }

    /// update last_transform and returns true if changed.
    fn update_last_transform(&mut self, position: Vec2, rotation: f32) -> bool {
        if let Some(last_transform) = self.last_transform {
            if last_transform.0.abs_diff_eq(position, F32_MAX_DIFF) &&
                    (last_transform.1 - rotation).abs() <= F32_MAX_DIFF {
                return false;
            }
        }
        self.last_transform = Some((position, rotation));
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
        Data::new().insert_with_limit("handle", Value::String(format!("{:?}", self.handle)), Limit::ReadOnly)
            .insert_with_limit("body_type", Value::Int32(self.body_type as i32),
                Limit::Int32Enum(vec![(0, "Dynamic".into()), (1, "Fixed".into()),
                (2, "KinematicPositionBased".into()), (3, "KinematicVelocityBased".into())]))
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Int32(i)) = data.get("body_type") { self.body_type = Self::i32_to_rigid_body_type(i) }
    }
}

/// A 2D collider in physics world of rapier2d.
#[derive(Component, Debug)]
#[track(All)]
pub struct Collider2D {
    handle: ColliderHandle,
    pub shape: ShapeWrapper,
    pub restitution: f32,
    pub sensor: bool,
    last_transform: Option<(Vec2, f32, Vec2)>, // translation, rotation and scale
}

impl Collider2D {
    /// Create a new Collider2D with shape, restitution, and sensor.
    pub fn new(shape: SharedShape, restitution: f32, sensor: bool) -> Self {
        Collider2D { handle: ColliderHandle::invalid(), shape: ShapeWrapper(shape), restitution, sensor, last_transform: None }
    }

    /// Get the raw handle of this collider.
    pub fn handle(&self) -> ColliderHandle {
        self.handle
    }

    /// update last_transform and returns true if changed.
    fn update_last_transform(&mut self, position: Vec2, rotation: f32, scale: Vec2) -> bool {
        if let Some(last_transform) = self.last_transform {
            if last_transform.0.abs_diff_eq(position, F32_MAX_DIFF) &&
                    (last_transform.1 - rotation).abs() <= F32_MAX_DIFF &&
                    last_transform.2.abs_diff_eq(scale, F32_MAX_DIFF) {
                return false;
            }
        }
        self.last_transform = Some((position, rotation, scale));
        true
    }

    /// Create a new shape scaled by transform.
    fn scaled_shape(&self, scale: Vec2) -> SharedShape {
        match self.shape.shape_type() {
            ShapeType::Ball => {
                let scale = std::cmp::max_by(scale.x.abs(), scale.y.abs(), |x, y| x.partial_cmp(y).unwrap());
                let radius = self.shape.as_ball().unwrap().radius * scale;
                SharedShape::new(Ball::new(radius))
            },
            ShapeType::Cuboid => {
                SharedShape::new(self.shape.as_cuboid().unwrap().scaled(&scale.into()))
            }
            _ => self.shape.clone(), // TODO: support all shape type
        }
    }
}

impl Default for Collider2D {
    fn default() -> Self {
        Self { handle: ColliderHandle::invalid(), shape: ShapeWrapper(SharedShape::cuboid(0.5, 0.5)), restitution: Default::default(), sensor: false, last_transform: None }
    }
}

impl Edit for Collider2D {
    fn name() -> &'static str { "Collider2D" }

    fn get_data(&self) -> Data {
        let mut data = Data::new().insert_with_limit("handle", Value::String(format!("{:?}", self.handle)), Limit::ReadOnly);
        self.shape.get_data(&mut data);
        data.insert("restitution", Value::Float32(self.restitution))
            .insert("sensor", Value::Bool(self.sensor))
    }

    fn set_data(&mut self, data: &Data) {
        self.shape.set_data(data);
        if let Some(Value::Float32(f)) = data.get("restitution") { self.restitution = *f };
        if let Some(Value::Bool(b)) = data.get("sensor") { self.sensor = *b };
    }
}

/// This unique contains all core objects in physics world of rapier2d.
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
    /// Call physics_pipeline.step to update the physics world.
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
        Physics2DManager {
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            gravity: vector![0.0, -9.81],
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            physics_hooks: Box::new(()),
            event_handler: Box::new(()),
            debug_render_pipeline: DebugRenderPipeline::default(),
        }
    }
}

impl Edit for Physics2DManager {
    fn name() -> &'static str { "Physics2DManager" }

    fn get_data(&self) -> Data {
        Data::new().insert("gravity", Value::Vec2(self.gravity.into()))
            // Currently integration_parameters.dt is dynamically changed so that it should not be edited
            .insert("min_ccd_dt", Value::Float32(self.integration_parameters.min_ccd_dt))
            .insert("erp", Value::Float32(self.integration_parameters.erp))
            .insert("damping_ratio", Value::Float32(self.integration_parameters.damping_ratio))
            .insert("joint_erp", Value::Float32(self.integration_parameters.joint_erp))
            .insert("joint_damping_ratio", Value::Float32(self.integration_parameters.joint_damping_ratio))
            .insert("allowed_linear_error", Value::Float32(self.integration_parameters.allowed_linear_error))
            .insert("max_penetration_correction", Value::Float32(self.integration_parameters.max_penetration_correction))
            .insert("prediction_distance", Value::Float32(self.integration_parameters.prediction_distance))
            .insert("max_velocity_iterations", Value::Int32(self.integration_parameters.max_velocity_iterations as i32))
            .insert("max_velocity_friction_iterations", Value::Int32(self.integration_parameters.max_velocity_friction_iterations as i32))
            .insert("max_stabilization_iterations", Value::Int32(self.integration_parameters.max_stabilization_iterations as i32))
            .insert("interleave_restitution_and_friction_resolution", Value::Bool(self.integration_parameters.interleave_restitution_and_friction_resolution))
            .insert("min_island_size", Value::Int32(self.integration_parameters.min_island_size as i32))
            .insert("max_ccd_substeps", Value::Int32(self.integration_parameters.max_ccd_substeps as i32))
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Vec2(v)) = data.get("gravity") { self.gravity = (*v).into() }
        if let Some(Value::Float32(v)) = data.get("min_ccd_dt") { self.integration_parameters.min_ccd_dt = *v }
        if let Some(Value::Float32(v)) = data.get("erp") { self.integration_parameters.erp = *v }
        if let Some(Value::Float32(v)) = data.get("damping_ratio") { self.integration_parameters.damping_ratio = *v }
        if let Some(Value::Float32(v)) = data.get("joint_erp") { self.integration_parameters.joint_erp = *v }
        if let Some(Value::Float32(v)) = data.get("joint_damping_ratio") { self.integration_parameters.joint_damping_ratio = *v }
        if let Some(Value::Float32(v)) = data.get("allowed_linear_error") { self.integration_parameters.allowed_linear_error = *v }
        if let Some(Value::Float32(v)) = data.get("max_penetration_correction") { self.integration_parameters.max_penetration_correction = *v }
        if let Some(Value::Float32(v)) = data.get("prediction_distance") { self.integration_parameters.prediction_distance = *v }
        if let Some(Value::Int32(v)) = data.get("max_velocity_iterations") { self.integration_parameters.max_velocity_iterations = *v as usize }
        if let Some(Value::Int32(v)) = data.get("max_velocity_friction_iterations") { self.integration_parameters.max_velocity_friction_iterations = *v as usize }
        if let Some(Value::Int32(v)) = data.get("max_stabilization_iterations") { self.integration_parameters.max_stabilization_iterations = *v as usize }
        if let Some(Value::Bool(v)) = data.get("interleave_restitution_and_friction_resolution") { self.integration_parameters.interleave_restitution_and_friction_resolution = *v }
        if let Some(Value::Int32(v)) = data.get("min_island_size") { self.integration_parameters.min_island_size = *v as usize }
        if let Some(Value::Int32(v)) = data.get("max_ccd_substeps") { self.integration_parameters.max_ccd_substeps = *v as usize }
    }
}

/// Modify, add, remove rigid bodies and colliders according to RigidBody2D and Collider2D components.
pub fn physics2d_maintain_system(mut physics2d_manager: UniqueViewMut<Physics2DManager>,
        mut rb2d: ViewMut<RigidBody2D>, mut col2d: ViewMut<Collider2D>,
        mut transforms: ViewMut<Transform>, children: View<Child>) {
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
            if !transforms.contains(e) {
                transforms.add_component_unchecked(e, Transform::default());
            }
            let model = Transform::entity_final_model(e, &children, &transforms).unwrap_or_default();
            let (_, rotation, position) = model.to_2d_scale_rotation_translation();
            let rigid_body = RigidBodyBuilder::new(rb2d.body_type)
                    .translation(position.into())
                    .rotation(rotation).build();
            rb2d.handle = physics2d_manager.rigid_body_set.insert(rigid_body);
        }

        if let Ok(col2d) = col2d.get(e) {
            if physics2d_manager.collider_set.contains(col2d.handle) {
                physics2d_manager.collider_set.set_parent(col2d.handle, Some(rb2d.handle), &mut physics2d_manager.rigid_body_set)
            }
        }
    }

    for (e, mut col2d) in col2d.inserted_or_modified_mut().iter().with_id() {
        if !transforms.contains(e) {
            transforms.add_component_unchecked(e, Transform::default());
        }
        let model = Transform::entity_final_model(e, &children, &transforms).unwrap_or_default();
        let (scale, rotation, position) = model.to_2d_scale_rotation_translation();
        let shape = col2d.scaled_shape(scale);

        if let Some(collider) = physics2d_manager.collider_set.get_mut(col2d.handle) {
            collider.set_shape(shape);
            collider.set_restitution(col2d.restitution);
            collider.set_sensor(col2d.sensor);
        } else {
            let mut collider = ColliderBuilder::new(shape).restitution(col2d.restitution).sensor(col2d.sensor).build();
            if let Ok(rb2d) = &rb2d.get(e) {
                // TODO: add position and rotation relative to parent
                col2d.handle = physics2d_manager.collider_set.insert_with_parent(collider, rb2d.handle, &mut physics2d_manager.rigid_body_set);
            } else {
                collider.set_translation(position.into());
                collider.set_rotation(Rotation::from_angle(rotation));
                col2d.handle = physics2d_manager.collider_set.insert(collider);
            }
        }
    }

    // also call this in maintain system for steel-editor when update systems do not run
    // TODO: do not call this when update systems are running
    physics2d_update_from_transform(physics2d_manager, &transforms, &children, &mut rb2d, &mut col2d);

    rb2d.clear_all_removed_and_deleted();
    col2d.clear_all_removed_and_deleted();
    rb2d.clear_all_inserted_and_modified();
    col2d.clear_all_inserted_and_modified();
}

/// Update rigid bodies and colliders according to Transform component.
fn physics2d_update_from_transform(physics2d_manager: &mut Physics2DManager, transforms: &ViewMut<Transform>,
        children: &View<Child>, rb2d: &mut ViewMut<RigidBody2D>, col2d: &mut ViewMut<Collider2D>) {
    // TODO: find a way to use par_iter here (&mut physics2d_manager.rigid_body_set can not be passed to par_iter)
    for (transform, child, mut rb2d) in (transforms, children, rb2d).iter() {
        let model = transform.final_model(child, children, transforms);
        let (_, rotation, position) = model.to_2d_scale_rotation_translation();
        if rb2d.update_last_transform(position, rotation) {
            if let Some(rb2d) = physics2d_manager.rigid_body_set.get_mut(rb2d.handle) {
                rb2d.set_translation(position.into(), true);
                rb2d.set_rotation(Rotation::from_angle(rotation), true);
            }
        }
    }

    for (transform, child, mut col2d) in (transforms, children, col2d).iter() {
        let model = transform.final_model(child, children, transforms);
        let (scale, rotation, position) = model.to_2d_scale_rotation_translation();
        if col2d.update_last_transform(position, rotation, scale) {
            if let Some(collider) = physics2d_manager.collider_set.get_mut(col2d.handle) {
                collider.set_shape(col2d.scaled_shape(scale));
                collider.set_translation(position.into());
                collider.set_rotation(Rotation::from_angle(rotation));
            }
        }
    }
}

/// Update physics world.
pub fn physics2d_update_system(mut physics2d_manager: UniqueViewMut<Physics2DManager>, time: UniqueView<Time>,
        mut rb2d: ViewMut<RigidBody2D>, mut col2d: ViewMut<Collider2D>, mut transforms: ViewMut<Transform>, children: View<Child>) {
    let physics2d_manager = physics2d_manager.as_mut();

    // dynamically change integration_parameters.dt according to time.delta()
    physics2d_manager.integration_parameters.dt = time.delta();

    physics2d_update_from_transform(physics2d_manager, &transforms, &children, &mut rb2d, &mut col2d);

    physics2d_manager.update();

    // TODO: find a way to use par_iter here
    for (e, (rb2d, child)) in (&rb2d, &children).iter().with_id() {
        let rigid_body = &physics2d_manager.rigid_body_set[rb2d.handle];

        // get transform
        if !transforms.contains(e) {
            transforms.add_component_unchecked(e, Transform::default());
        }
        let transform = transforms.get(e).unwrap();

        // get old final model and decompose
        let final_model = transform.final_model(child, &children, &transforms);
        let (scale, mut rotation, mut position) = final_model.to_scale_rotation_translation();

        // update position and rotation
        position.x = rigid_body.translation().x;
        position.y = rigid_body.translation().y;
        let mut rot = rotation.to_scaled_axis();
        rot.z = rigid_body.rotation().angle();
        rotation = Quat::from_scaled_axis(rot);

        // get new final model
        let final_model = Mat4::from_scale_rotation_translation(scale, rotation, position);

        // get new model
        let model = if let Some(parent_final_model) = Transform::entity_final_model(child.parent(), &children, &transforms) {
            // parent_final_model * model = final_model => model = parent_final_model.inverse() * final_model
            parent_final_model.inverse() * final_model
        } else {
            final_model
        };

        // update transform according to new model
        let (scale, rotation, position) = model.to_scale_rotation_translation();
        let transform = (&mut transforms).get(e).unwrap();
        transform.position = position;
        transform.rotation = rotation;
        transform.scale = scale;
    };

    // update last_transform using final model
    (&mut rb2d, &transforms, &children).par_iter().for_each(|(mut rb2d, transform, child)| {
        let model = transform.final_model(child, &children, &transforms);
        let (_, rotation, position) = model.to_2d_scale_rotation_translation();
        rb2d.update_last_transform(position, rotation);
    });
    (&mut col2d, &transforms, &children).par_iter().for_each(|(mut col2d, transform, child)| {
        let model = transform.final_model(child, &children, &transforms);
        let (scale, rotation, position) = model.to_2d_scale_rotation_translation();
        col2d.update_last_transform(position, rotation, scale);
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

/// Drawing physics debug lines to the Canvas.
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
