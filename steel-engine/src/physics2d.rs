use crate::{
    app::{Plugin, Schedule, SteelApp},
    edit::Edit,
    hierarchy::Parent,
    render::canvas::Canvas,
    shape2d::Shape2D,
    time::Time,
    transform::Transform,
};
use glam::{Affine2, Quat, Vec2, Vec3, Vec4};
use rapier2d::prelude::*;
use shipyard::{
    AddComponent, Component, EntityId, Get, IntoIter, IntoWithId, Unique, UniqueView,
    UniqueViewMut, View, ViewMut,
};
use std::{collections::HashMap, num::NonZero};
use steel_common::data::{Data, Limit, Value};

/// Maximum difference between two f32 values to be considered equal.
pub const F32_MAX_DIFF: f32 = 0.0001;

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
        RigidBody2D {
            handle: RigidBodyHandle::invalid(),
            body_type,
            last_transform: None,
        }
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
            if last_transform.0.abs_diff_eq(position, F32_MAX_DIFF)
                && (last_transform.1 - rotation).abs() <= F32_MAX_DIFF
            {
                return false;
            }
        }
        self.last_transform = Some((position, rotation));
        true
    }
}

impl Default for RigidBody2D {
    fn default() -> Self {
        Self {
            handle: RigidBodyHandle::invalid(),
            body_type: RigidBodyType::Dynamic,
            last_transform: None,
        }
    }
}

impl Edit for RigidBody2D {
    fn name() -> &'static str {
        "RigidBody2D"
    }

    fn get_data(&self) -> Data {
        Data::new()
            .insert_with_limit(
                "handle",
                Value::String(format!("{:?}", self.handle)),
                Limit::ReadOnly,
            )
            .insert_with_limit(
                "body_type",
                Value::Int32(self.body_type as i32),
                Limit::Int32Enum(vec![
                    (0, "Dynamic".into()),
                    (1, "Fixed".into()),
                    (2, "KinematicPositionBased".into()),
                    (3, "KinematicVelocityBased".into()),
                ]),
            )
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Int32(i)) = data.get("body_type") {
            self.body_type = Self::i32_to_rigid_body_type(i)
        }
    }
}

/// A 2D collider in physics world of rapier2d.
#[derive(Component, Debug)]
#[track(All)]
pub struct Collider2D {
    handle: ColliderHandle,
    pub shape: Shape2D,
    pub restitution: f32,
    pub sensor: bool,
    last_transform: Option<(Vec2, f32, Vec2)>, // translation, rotation and scale
}

impl Collider2D {
    /// Create a new Collider2D with shape, restitution, and sensor.
    pub fn new(shape: SharedShape, restitution: f32, sensor: bool) -> Self {
        Collider2D {
            handle: ColliderHandle::invalid(),
            shape: Shape2D(shape),
            restitution,
            sensor,
            last_transform: None,
        }
    }

    /// Get the raw handle of this collider.
    pub fn handle(&self) -> ColliderHandle {
        self.handle
    }

    /// update last_transform and returns true if changed.
    fn update_last_transform(&mut self, position: Vec2, rotation: f32, scale: Vec2) -> bool {
        if let Some(last_transform) = self.last_transform {
            if last_transform.0.abs_diff_eq(position, F32_MAX_DIFF)
                && (last_transform.1 - rotation).abs() <= F32_MAX_DIFF
                && last_transform.2.abs_diff_eq(scale, F32_MAX_DIFF)
            {
                return false;
            }
        }
        self.last_transform = Some((position, rotation, scale));
        true
    }

    /// Create a new shape scaled by scale value.
    fn scaled_shape(&self, scale: Vec2) -> SharedShape {
        match self.shape.shape_type() {
            ShapeType::Ball => {
                let scale = std::cmp::max_by(scale.x.abs(), scale.y.abs(), |x, y| {
                    x.partial_cmp(y).unwrap()
                });
                let radius = self.shape.as_ball().unwrap().radius * scale;
                SharedShape::new(Ball::new(radius))
            }
            ShapeType::Cuboid => {
                SharedShape::new(self.shape.as_cuboid().unwrap().scaled(&scale.into()))
            }
            _ => self.shape.clone(), // TODO: support all shape type
        }
    }
}

impl Default for Collider2D {
    fn default() -> Self {
        Self {
            handle: ColliderHandle::invalid(),
            shape: Shape2D(SharedShape::cuboid(0.5, 0.5)),
            restitution: Default::default(),
            sensor: false,
            last_transform: None,
        }
    }
}

impl Edit for Collider2D {
    fn name() -> &'static str {
        "Collider2D"
    }

    fn get_data(&self) -> Data {
        let mut data = Data::new().insert_with_limit(
            "handle",
            Value::String(format!("{:?}", self.handle)),
            Limit::ReadOnly,
        );
        self.shape.get_data(&mut data);
        data.insert("restitution", Value::Float32(self.restitution))
            .insert("sensor", Value::Bool(self.sensor))
    }

    fn set_data(&mut self, data: &Data) {
        self.shape.set_data(data);
        if let Some(Value::Float32(f)) = data.get("restitution") {
            self.restitution = *f
        };
        if let Some(Value::Bool(b)) = data.get("sensor") {
            self.sensor = *b
        };
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
    pub broad_phase: DefaultBroadPhase,
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
            broad_phase: DefaultBroadPhase::new(),
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
    fn name() -> &'static str {
        "Physics2DManager"
    }

    fn get_data(&self) -> Data {
        Data::new()
            .insert("gravity", Value::Vec2(self.gravity.into()))
            // Currently integration_parameters.dt is dynamically changed so that it should not be edited
            .insert(
                "min_ccd_dt",
                Value::Float32(self.integration_parameters.min_ccd_dt),
            )
            .insert(
                "contact_damping_ratio",
                Value::Float32(self.integration_parameters.contact_damping_ratio),
            )
            .insert(
                "contact_natural_frequency",
                Value::Float32(self.integration_parameters.contact_natural_frequency),
            )
            .insert(
                "joint_natural_frequency",
                Value::Float32(self.integration_parameters.joint_natural_frequency),
            )
            .insert(
                "joint_damping_ratio",
                Value::Float32(self.integration_parameters.joint_damping_ratio),
            )
            .insert(
                "warmstart_coefficient",
                Value::Float32(self.integration_parameters.warmstart_coefficient),
            )
            .insert(
                "length_unit",
                Value::Float32(self.integration_parameters.length_unit),
            )
            .insert(
                "normalized_allowed_linear_error",
                Value::Float32(self.integration_parameters.normalized_allowed_linear_error),
            )
            .insert(
                "normalized_max_corrective_velocity",
                Value::Float32(
                    self.integration_parameters
                        .normalized_max_corrective_velocity,
                ),
            )
            .insert(
                "normalized_prediction_distance",
                Value::Float32(self.integration_parameters.normalized_prediction_distance),
            )
            .insert_with_limit(
                "num_solver_iterations",
                Value::UInt32(self.integration_parameters.num_solver_iterations.get() as u32),
                Limit::UInt32Range(1..=u32::MAX),
            )
            .insert(
                "num_additional_friction_iterations",
                Value::UInt32(
                    self.integration_parameters
                        .num_additional_friction_iterations as u32,
                ),
            )
            .insert(
                "num_internal_pgs_iterations",
                Value::UInt32(self.integration_parameters.num_internal_pgs_iterations as u32),
            )
            .insert(
                "num_internal_stabilization_iterations",
                Value::UInt32(
                    self.integration_parameters
                        .num_internal_stabilization_iterations as u32,
                ),
            )
            .insert(
                "min_island_size",
                Value::UInt32(self.integration_parameters.min_island_size as u32),
            )
            .insert(
                "max_ccd_substeps",
                Value::UInt32(self.integration_parameters.max_ccd_substeps as u32),
            )
    }

    fn set_data(&mut self, data: &Data) {
        if let Some(Value::Vec2(v)) = data.get("gravity") {
            self.gravity = (*v).into()
        }
        if let Some(Value::Float32(v)) = data.get("min_ccd_dt") {
            self.integration_parameters.min_ccd_dt = *v
        }
        if let Some(Value::Float32(v)) = data.get("contact_damping_ratio") {
            self.integration_parameters.contact_damping_ratio = *v
        }
        if let Some(Value::Float32(v)) = data.get("contact_natural_frequency") {
            self.integration_parameters.contact_natural_frequency = *v
        }
        if let Some(Value::Float32(v)) = data.get("joint_natural_frequency") {
            self.integration_parameters.joint_natural_frequency = *v
        }
        if let Some(Value::Float32(v)) = data.get("joint_damping_ratio") {
            self.integration_parameters.joint_damping_ratio = *v
        }
        if let Some(Value::Float32(v)) = data.get("warmstart_coefficient") {
            self.integration_parameters.warmstart_coefficient = *v
        }
        if let Some(Value::Float32(v)) = data.get("length_unit") {
            self.integration_parameters.length_unit = *v
        }
        if let Some(Value::Float32(v)) = data.get("normalized_allowed_linear_error") {
            self.integration_parameters.normalized_allowed_linear_error = *v
        }
        if let Some(Value::Float32(v)) = data.get("normalized_max_corrective_velocity") {
            self.integration_parameters
                .normalized_max_corrective_velocity = *v
        }
        if let Some(Value::Float32(v)) = data.get("normalized_prediction_distance") {
            self.integration_parameters.normalized_prediction_distance = *v
        }
        if let Some(Value::UInt32(v)) = data.get("num_solver_iterations") {
            self.integration_parameters.num_solver_iterations =
                NonZero::new(1.max(*v) as usize).unwrap()
        }
        if let Some(Value::UInt32(v)) = data.get("num_additional_friction_iterations") {
            self.integration_parameters
                .num_additional_friction_iterations = *v as usize
        }
        if let Some(Value::UInt32(v)) = data.get("num_internal_pgs_iterations") {
            self.integration_parameters.num_internal_pgs_iterations = *v as usize
        }
        if let Some(Value::UInt32(v)) = data.get("num_internal_stabilization_iterations") {
            self.integration_parameters
                .num_internal_stabilization_iterations = *v as usize
        }
        if let Some(Value::UInt32(v)) = data.get("min_island_size") {
            self.integration_parameters.min_island_size = *v as usize
        }
        if let Some(Value::UInt32(v)) = data.get("max_ccd_substeps") {
            self.integration_parameters.max_ccd_substeps = *v as usize
        }
    }
}

/// Modify, add, remove rigid bodies and colliders according to RigidBody2D and Collider2D components.
pub fn physics2d_maintain_system(
    mut physics2d_manager: UniqueViewMut<Physics2DManager>,
    mut rb2d: ViewMut<RigidBody2D>,
    mut col2d: ViewMut<Collider2D>,
    mut transforms: ViewMut<Transform>,
    parents: View<Parent>,
) {
    let physics2d_manager = physics2d_manager.as_mut();

    for e in rb2d.removed() {
        log::warn!(
            "Leak warning: RigidBody2D component of entity({e:?}) has been removed, \
            we don't know its handle so that its body can not be removed from physics world! \
            please use delete instead of remove on RigidBody2D component"
        );
    }

    for e in col2d.removed() {
        log::warn!(
            "Leak warning: Collider2D component of entity({e:?}) has been removed, \
            we don't know its handle so that its body can not be removed from physics world! \
            please use delete instead of remove on Collider2D component"
        );
    }

    for (_, rb2d) in rb2d.deleted() {
        if physics2d_manager.rigid_body_set.contains(rb2d.handle) {
            physics2d_manager.rigid_body_set.remove(
                rb2d.handle,
                &mut physics2d_manager.island_manager,
                &mut physics2d_manager.collider_set,
                &mut physics2d_manager.impulse_joint_set,
                &mut physics2d_manager.multibody_joint_set,
                false,
            );
        }
    }

    for (_, col2d) in col2d.deleted() {
        if physics2d_manager.collider_set.contains(col2d.handle) {
            physics2d_manager.collider_set.remove(
                col2d.handle,
                &mut physics2d_manager.island_manager,
                &mut physics2d_manager.rigid_body_set,
                true,
            );
        }
    }

    let mut model_cache = Some(HashMap::new());
    for (e, rb2d) in rb2d.inserted_or_modified_mut().iter().with_id() {
        if let Some(rigid_body) = physics2d_manager.rigid_body_set.get_mut(rb2d.handle) {
            rigid_body.set_body_type(rb2d.body_type, true);
        } else {
            if !transforms.contains(e) {
                transforms.add_component_unchecked(e, Transform::default());
            }
            let model_without_scale = Transform::entity_final_model_without_scale_2d(
                e,
                &parents,
                &transforms,
                &mut model_cache,
            )
            .unwrap();
            let (_, rotation, position) = model_without_scale.to_scale_angle_translation();
            let rigid_body = RigidBodyBuilder::new(rb2d.body_type)
                .translation(position.into())
                .rotation(rotation)
                .build();
            rb2d.handle = physics2d_manager.rigid_body_set.insert(rigid_body);
        }

        if let Ok(col2d) = col2d.get(e) {
            if physics2d_manager.collider_set.contains(col2d.handle) {
                physics2d_manager.collider_set.set_parent(
                    col2d.handle,
                    Some(rb2d.handle),
                    &mut physics2d_manager.rigid_body_set,
                )
            }
        }
    }

    let mut scale_cache = Some(HashMap::new());
    for (e, col2d) in col2d.inserted_or_modified_mut().iter().with_id() {
        if !transforms.contains(e) {
            transforms.add_component_unchecked(e, Transform::default());
        }
        let scale =
            Transform::entity_final_scale_2d(e, &parents, &transforms, &mut scale_cache).unwrap();
        let shape = col2d.scaled_shape(scale);

        if let Some(collider) = physics2d_manager.collider_set.get_mut(col2d.handle) {
            collider.set_shape(shape);
            collider.set_restitution(col2d.restitution);
            collider.set_sensor(col2d.sensor);
        } else {
            let mut collider = ColliderBuilder::new(shape)
                .restitution(col2d.restitution)
                .sensor(col2d.sensor)
                .build();
            if let Ok(rb2d) = &rb2d.get(e) {
                // TODO: add position and rotation relative to parent
                col2d.handle = physics2d_manager.collider_set.insert_with_parent(
                    collider,
                    rb2d.handle,
                    &mut physics2d_manager.rigid_body_set,
                );
            } else {
                let model_without_scale = Transform::entity_final_model_without_scale_2d(
                    e,
                    &parents,
                    &transforms,
                    &mut model_cache,
                )
                .unwrap();
                let (_, rotation, position) = model_without_scale.to_scale_angle_translation();
                collider.set_translation(position.into());
                collider.set_rotation(Rotation::from_angle(rotation));
                col2d.handle = physics2d_manager.collider_set.insert(collider);
            }
        }
    }

    // also call this in maintain system for steel-editor when update systems do not run
    // TODO: do not call this when update systems are running
    physics2d_update_from_transform(
        physics2d_manager,
        &transforms,
        &parents,
        &mut rb2d,
        &mut col2d,
    );

    rb2d.clear_all_removed_and_deleted();
    col2d.clear_all_removed_and_deleted();
    rb2d.clear_all_inserted_and_modified();
    col2d.clear_all_inserted_and_modified();
}

/// Update rigid bodies and colliders according to Transform component.
fn physics2d_update_from_transform(
    physics2d_manager: &mut Physics2DManager,
    transforms: &ViewMut<Transform>,
    parents: &View<Parent>,
    rb2d: &mut ViewMut<RigidBody2D>,
    col2d: &mut ViewMut<Collider2D>,
) {
    let mut model_cache = Some(HashMap::new());
    for (e, (mut rb2d, _)) in (rb2d, transforms).iter().with_id() {
        // we can safely unwrap here because we know that this entity has a Transform component
        let model_without_scale = Transform::entity_final_model_without_scale_2d(
            e,
            parents,
            transforms,
            &mut model_cache,
        )
        .unwrap();
        let (_, rotation, position) = model_without_scale.to_scale_angle_translation();
        if rb2d.update_last_transform(position, rotation) {
            if let Some(rb2d) = physics2d_manager.rigid_body_set.get_mut(rb2d.handle) {
                rb2d.set_translation(position.into(), true);
                rb2d.set_rotation(Rotation::from_angle(rotation), true);
            }
        }
    }

    let mut scale_cache = Some(HashMap::new());
    for (e, (mut col2d, _)) in (col2d, transforms).iter().with_id() {
        // we can safely unwrap here because we know that this entity has a Transform component
        let model_without_scale = Transform::entity_final_model_without_scale_2d(
            e,
            parents,
            transforms,
            &mut model_cache,
        )
        .unwrap();
        let (_, rotation, position) = model_without_scale.to_scale_angle_translation();
        let scale =
            Transform::entity_final_scale_2d(e, parents, transforms, &mut scale_cache).unwrap();
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
pub fn physics2d_update_system(
    mut physics2d_manager: UniqueViewMut<Physics2DManager>,
    time: UniqueView<Time>,
    mut rb2d: ViewMut<RigidBody2D>,
    mut col2d: ViewMut<Collider2D>,
    mut transforms: ViewMut<Transform>,
    parents: View<Parent>,
) {
    let physics2d_manager = physics2d_manager.as_mut();

    // dynamically change integration_parameters.dt according to time.delta()
    physics2d_manager.integration_parameters.dt = if time.delta() < 0.1 {
        time.delta()
    } else {
        0.1
    };

    physics2d_update_from_transform(
        physics2d_manager,
        &transforms,
        &parents,
        &mut rb2d,
        &mut col2d,
    );

    physics2d_manager.update();

    let mut model_cache = Some(HashMap::new());
    let mut final_position_and_rotation = HashMap::new();
    for (e, rb2d) in rb2d.iter().with_id() {
        // get new final_position and final_rotation from rigid_body and cache them
        let rigid_body = &physics2d_manager.rigid_body_set[rb2d.handle];
        let final_position: Vec2 = (*rigid_body.translation()).into();
        let final_rotation = rigid_body.rotation().angle();
        final_position_and_rotation.insert(e, (final_position, final_rotation));

        // caculate new final_model_without_scale from final_position and final_rotation, then cache result
        let final_model_without_scale =
            Affine2::from_angle_translation(final_rotation, final_position);
        model_cache
            .as_mut()
            .unwrap()
            .insert(e, Some(final_model_without_scale));
    }

    for (e, _) in rb2d.iter().with_id() {
        // get parent_final_model_without_scale by using cache
        let parent = parents.get(e).map(|p| **p).unwrap_or_default();
        let parent_final_model_without_scale = Transform::entity_final_model_without_scale_2d(
            parent,
            &parents,
            &transforms,
            &mut model_cache,
        );

        // caculate new position and rotation according to final_model_without_scale and parent_final_model_without_scale
        let (position, rotation) = if let Some(parent_final_model_without_scale) =
            parent_final_model_without_scale
        {
            // get final_model_without_scale from cache
            let final_model_without_scale = model_cache.as_ref().unwrap().get(&e).unwrap().unwrap();

            // parent_final_model_without_scale * model_without_scale = final_model_without_scale =>
            // model_without_scale = parent_final_model_without_scale.inverse() * final_model_without_scale
            let model_without_scale =
                parent_final_model_without_scale.inverse() * final_model_without_scale;
            let (_, rotation, position) = model_without_scale.to_scale_angle_translation();
            (position, rotation)
        } else {
            // get final_position and final_rotation from cache
            *final_position_and_rotation.get(&e).unwrap()
        };

        // update transform to new position and rotation
        if !transforms.contains(e) {
            transforms.add_component_unchecked(e, Transform::default());
        }
        let mut transform = (&mut transforms).get(e).unwrap();
        transform.position.x = position.x;
        transform.position.y = position.y;
        let mut rot = transform.rotation.to_scaled_axis();
        rot.z = rotation;
        transform.rotation = Quat::from_scaled_axis(rot);
    }

    // update last transform

    // we must use a new model_cache here, or physics2d_update_from_transform will
    // modify rigid bodies and colliders due to the imprecision of floating point numbers
    let mut model_cache = Some(HashMap::new());
    for (e, (mut rb2d, _)) in (&mut rb2d, &transforms).iter().with_id() {
        let model_without_scale = Transform::entity_final_model_without_scale_2d(
            e,
            &parents,
            &transforms,
            &mut model_cache,
        )
        .unwrap();
        let (_, rotation, position) = model_without_scale.to_scale_angle_translation();
        rb2d.update_last_transform(position, rotation);
    }

    let mut scale_cache = Some(HashMap::new());
    for (e, (mut col2d, _)) in (&mut col2d, &transforms).iter().with_id() {
        let model_without_scale = Transform::entity_final_model_without_scale_2d(
            e,
            &parents,
            &transforms,
            &mut model_cache,
        )
        .unwrap();
        let (_, rotation, position) = model_without_scale.to_scale_angle_translation();
        let scale =
            Transform::entity_final_scale_2d(e, &parents, &transforms, &mut scale_cache).unwrap();
        col2d.update_last_transform(position, rotation, scale);
    }
}

struct DebugRenderer<'a> {
    canvas: &'a mut Canvas,
}

impl DebugRenderBackend for DebugRenderer<'_> {
    fn draw_line(&mut self, _: DebugRenderObject, a: Point<Real>, b: Point<Real>, color: [f32; 4]) {
        // currently we use a big z value to make sure that debug render content can be seen
        // TODO: find a better way to make sure the visiblity of debug render content
        let color = Vec4::from_array(color);
        self.canvas.line(
            Vec3::new(a.x, a.y, 1000.0),
            Vec3::new(b.x, b.y, 1000.0),
            color,
            EntityId::dead(),
        ); // TODO: eid
    }
}

/// Drawing physics debug lines to the Canvas.
pub fn physics2d_debug_render_system(
    mut physics2d_manager: UniqueViewMut<Physics2DManager>,
    mut canvas: UniqueViewMut<Canvas>,
) {
    let physics2d_manager = physics2d_manager.as_mut();
    let mut debug_render_backend = DebugRenderer {
        canvas: &mut canvas,
    };
    physics2d_manager.debug_render_pipeline.render(
        &mut debug_render_backend,
        &physics2d_manager.rigid_body_set,
        &physics2d_manager.collider_set,
        &physics2d_manager.impulse_joint_set,
        &physics2d_manager.multibody_joint_set,
        &physics2d_manager.narrow_phase,
    );
}

/// The physics2d plugin. This plugin contains:
/// - [Physics2DManager]
/// - [RigidBody2D]
/// - [Collider2D]
/// - [physics2d_maintain_system]
/// - [physics2d_update_system]
/// - [physics2d_debug_render_system]
pub struct Physics2DPlugin;

impl Plugin for Physics2DPlugin {
    fn apply(self, app: SteelApp) -> SteelApp {
        app.add_and_register_unique(Physics2DManager::default())
            .register_component::<RigidBody2D>()
            .register_component::<Collider2D>()
            .add_system(
                Schedule::PreUpdate,
                crate::physics2d::physics2d_maintain_system,
            )
            .add_system(Schedule::Update, crate::physics2d::physics2d_update_system)
            .add_system(
                Schedule::DrawEditor,
                crate::physics2d::physics2d_debug_render_system,
            )
    }
}
