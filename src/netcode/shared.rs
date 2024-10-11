use avian3d::{
    prelude::{
        CoefficientCombine, Collider, ExternalForce, ExternalImpulse, Friction, LinearVelocity,
        LockedAxes, Mass, Physics, PhysicsSet, Position, RigidBody, SpatialQuery,
        SpatialQueryFilter,
    },
    sync::SyncPlugin,
    PhysicsPlugins,
};
use bevy::prelude::Dir3;
use bevy::prelude::Res;
use bevy::{
    app::{FixedUpdate, Plugin, PostUpdate},
    color::Color,
    math::Vec3,
    prelude::{Bundle, IntoSystemSetConfigs, SystemSet},
    render::RenderPlugin,
    time::Time,
};
use bevy::{
    ecs::query::QueryData,
    prelude::{Entity, PluginGroup},
};
use leafwing_input_manager::prelude::ActionState;
use lightyear::prelude::ClientId;

use crate::netcode::protocol::CharacterAction;
use crate::netcode::protocol::ProtocolPlugin;
use crate::render::ZinnobreIronRenderPlugin;

pub(crate) const CHARACTER_CAPSULE_RADIUS: f32 = 0.5;
pub(crate) const CHARACTER_CAPSULE_HEIGHT: f32 = 0.5;

#[derive(Bundle)]
pub(crate) struct CharacterPhysicsBundle {
    collider: Collider,
    rigid_body: RigidBody,
    external_force: ExternalForce,
    external_impulse: ExternalImpulse,
    lock_axes: LockedAxes,
    friction: Friction,
}

impl Default for CharacterPhysicsBundle {
    fn default() -> Self {
        Self {
            collider: Collider::capsule(CHARACTER_CAPSULE_RADIUS, CHARACTER_CAPSULE_HEIGHT),
            rigid_body: RigidBody::Dynamic,
            external_force: ExternalForce::ZERO.with_persistence(false),
            external_impulse: ExternalImpulse::ZERO.with_persistence(false),
            lock_axes: LockedAxes::default()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z(),
            friction: Friction::new(0.0).with_combine_rule(CoefficientCombine::Min),
        }
    }
}

pub(crate) const FLOOR_WIDTH: f32 = 100.0;
pub(crate) const FLOOR_HEIGHT: f32 = 1.0;
pub(crate) const FLOOR_LENGTH: f32 = 100.0;

#[derive(Bundle)]
pub(crate) struct FloorPhysicsBundle {
    collider: Collider,
    rigid_body: RigidBody,
}

impl Default for FloorPhysicsBundle {
    fn default() -> Self {
        Self {
            collider: Collider::cuboid(FLOOR_WIDTH, FLOOR_HEIGHT, FLOOR_LENGTH),
            rigid_body: RigidBody::Static,
        }
    }
}

pub(crate) const BLOCK_WIDTH: f32 = 1.0;
pub(crate) const BLOCK_HEIGHT: f32 = 1.0;
pub(crate) const BLOCK_LENGTH: f32 = 1.0;

#[derive(Bundle)]
pub(crate) struct BlockPhysicsBundle {
    collider: Collider,
    rigid_body: RigidBody,
}

impl Default for BlockPhysicsBundle {
    fn default() -> Self {
        Self {
            collider: Collider::cuboid(BLOCK_WIDTH, BLOCK_HEIGHT, BLOCK_LENGTH),
            rigid_body: RigidBody::Dynamic,
        }
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum FixedSet {
    // Main fixed update systems (i.e. inputs)
    Main,
    // Apply physics steps
    Physics,
}

#[derive(Clone)]
pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(ProtocolPlugin);
        if app.is_plugin_added::<RenderPlugin>() {
            app.add_plugins(ZinnobreIronRenderPlugin);
        }

        // Physics

        // Position and Rotation are the primary source of truth so no need to sync changes
        app.insert_resource(avian3d::sync::SyncConfig {
            transform_to_position: false,
            position_to_transform: true,
        });

        // We change SyncPlugin to PostUpdate, because we want the
        // visually interpreted values synced to transform every time,
        // not just when Fixed schedule runs.
        app.add_plugins(
            PhysicsPlugins::new(FixedUpdate)
                .build()
                .disable::<SyncPlugin>(),
        )
        .add_plugins(SyncPlugin::new(PostUpdate));

        const FIXED_TIMESTEP_HZ: f64 = 64.0;
        app.insert_resource(Time::new_with(Physics::fixed_once_hz(FIXED_TIMESTEP_HZ)));

        app.configure_sets(
            FixedUpdate,
            (
                // Make sure any physics simulation happens after Main
                (
                    PhysicsSet::Prepare,
                    PhysicsSet::StepSimulation,
                    PhysicsSet::Sync,
                )
                    .in_set(FixedSet::Physics),
                (FixedSet::Main, FixedSet::Physics).chain(),
            ),
        );
    }
}

// Generate player color based on id
pub(crate) fn color_from_id(client_id: ClientId) -> Color {
    let h = (((client_id.to_bits().wrapping_mul(30)) % 360) as f32) / 360.0;
    let s = 1.0;
    let l = 0.5;
    Color::hsl(h, s, l)
}

#[derive(QueryData)]
#[query_data(mutable, derive(Debug))]
pub struct CharacterQuery {
    pub external_force: &'static mut ExternalForce,
    pub external_impulse: &'static mut ExternalImpulse,
    pub linear_velocity: &'static LinearVelocity,
    pub mass: &'static Mass,
    pub position: &'static Position,
    pub entity: Entity,
}

pub fn apply_character_action(
    time: &Res<Time>,
    spatial_query: &SpatialQuery,
    action_state: &ActionState<CharacterAction>,
    character: &mut CharacterQueryItem,
) {
    const MAX_SPEED: f32 = 5.5;
    const MAX_ACCELERATION: f32 = 20.0;

    let max_velocity_delta_per_tick = MAX_ACCELERATION * time.delta_seconds();

    if action_state.pressed(&CharacterAction::Jump) {
        let ray_cast_origin = character.position.0
            + Vec3::new(
                0.0,
                -CHARACTER_CAPSULE_HEIGHT / 2.0 - CHARACTER_CAPSULE_RADIUS,
                0.0,
            );

        if spatial_query
            .cast_ray(
                ray_cast_origin,
                Dir3::NEG_Y,
                0.01,
                true,
                SpatialQueryFilter::from_excluded_entities([character.entity]),
            )
            .is_some()
        {
            character
                .external_impulse
                .apply_impulse(Vec3::new(0.0, 5.0, 0.0));
        }
    }

    let move_dir = action_state
        .axis_pair(&CharacterAction::Move)
        .clamp_length_max(1.0);
    let move_dir = Vec3::new(-move_dir.x, 0.0, move_dir.y);

    let ground_linear_velocity = Vec3::new(
        character.linear_velocity.x,
        0.0,
        character.linear_velocity.z,
    );

    let desired_ground_linear_velocity = move_dir * MAX_SPEED;

    let new_ground_linear_velocity = ground_linear_velocity
        .move_towards(desired_ground_linear_velocity, max_velocity_delta_per_tick);

    let required_acceleration =
        (new_ground_linear_velocity - ground_linear_velocity) / time.delta_seconds();

    character
        .external_force
        .apply_force(required_acceleration * character.mass.0);
}
