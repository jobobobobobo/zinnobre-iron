use crate::netcode::protocol::{BlockMarker, CharacterMarker, ColorComponent, FloorMarker};
use crate::netcode::shared::{
    BLOCK_HEIGHT, BLOCK_LENGTH, BLOCK_WIDTH, CHARACTER_CAPSULE_HEIGHT, CHARACTER_CAPSULE_RADIUS,
    FLOOR_HEIGHT, FLOOR_LENGTH, FLOOR_WIDTH,
};
use avian3d::prelude::{Position, Rotation};
use bevy::color::Color;
use bevy::prelude::Cuboid;
use bevy::{
    app::{Plugin, Startup, Update},
    asset::Assets,
    log::{debug, info},
    math::{Dir3, Vec3},
    pbr::{PbrBundle, PointLight, PointLightBundle, StandardMaterial},
    prelude::{
        default, Added, Camera3dBundle, Capsule3d, Commands, Component, Entity, Mesh, OnAdd, Query,
        ResMut, Transform, Trigger, With, Without,
    },
};
use bevy_screen_diagnostics::{
    Aggregate, ScreenDiagnostics, ScreenDiagnosticsPlugin, ScreenEntityDiagnosticsPlugin,
};
use lightyear::prelude::Replicated;
use lightyear::{
    client::prediction::diagnostics::PredictionDiagnosticsPlugin,
    prelude::client::{Confirmed, Predicted, VisualInterpolateStatus, VisualInterpolationPlugin},
    transport::io::IoDiagnosticsPlugin,
};

pub struct ZinnobreIronRenderPlugin;

impl Plugin for ZinnobreIronRenderPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Startup, init);
        app.add_systems(
            Update,
            (
                add_character_cosmetics,
                add_floor_cosmetics,
                add_block_cosmetics,
            ),
        );

        app.add_plugins(ScreenDiagnosticsPlugin::default());
        app.add_plugins(ScreenEntityDiagnosticsPlugin);

        app.add_plugins(VisualInterpolationPlugin::<Position>::default());
        app.add_plugins(VisualInterpolationPlugin::<Rotation>::default());

        app.observe(add_visual_interpolation_components::<Position>);
        app.observe(add_visual_interpolation_components::<Rotation>);
    }
}

fn init(mut commands: Commands, mut onscreen: ResMut<ScreenDiagnostics>) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 4.5, -9.0).looking_at(Vec3::ZERO, Dir3::Y),
        ..default()
    });

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    onscreen
        .add("RB".to_string(), PredictionDiagnosticsPlugin::ROLLBACKS)
        .aggregate(Aggregate::Value)
        .format(|v| format!("{v:.0}"));
    onscreen
        .add(
            "RBt".to_string(),
            PredictionDiagnosticsPlugin::ROLLBACK_TICKS,
        )
        .aggregate(Aggregate::Value)
        .format(|v| format!("{v:.0}"));
    onscreen
        .add(
            "RBd".to_string(),
            PredictionDiagnosticsPlugin::ROLLBACK_DEPTH,
        )
        .aggregate(Aggregate::Value)
        .format(|v| format!("{v:.1}"));

    onscreen
        .add("KB_in".to_string(), IoDiagnosticsPlugin::BYTES_IN)
        .aggregate(Aggregate::Average)
        .format(|v| format!("{v:0>3.0}"));
    onscreen
        .add("KB_out".to_string(), IoDiagnosticsPlugin::BYTES_OUT)
        .aggregate(Aggregate::Average)
        .format(|v| format!("{v:0>3.0}"));
}

fn add_visual_interpolation_components<T: Component>(
    trigger: Trigger<OnAdd, T>,
    query: Query<Entity, (With<T>, Without<Confirmed>, Without<FloorMarker>)>,
    mut commands: Commands,
) {
    if !query.contains(trigger.entity()) {
        return;
    }
    debug!("Adding visual interp component to {:?}", trigger.entity());
    commands
        .entity(trigger.entity())
        .insert(VisualInterpolateStatus::<T> {
            trigger_change_detection: true,
            ..default()
        });
}

fn add_character_cosmetics(
    mut commands: Commands,
    character_query: Query<(Entity, &ColorComponent), (Added<Predicted>, With<CharacterMarker>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, color) in &character_query {
        info!(?entity, "Adding cosmetics to character {:?}", entity);
        commands.entity(entity).insert((PbrBundle {
            mesh: meshes.add(Capsule3d::new(
                CHARACTER_CAPSULE_RADIUS,
                CHARACTER_CAPSULE_HEIGHT,
            )),
            material: materials.add(color.0),
            ..default()
        },));
    }
}

fn add_floor_cosmetics(
    mut commands: Commands,
    floor_query: Query<Entity, (Added<Replicated>, With<FloorMarker>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for entity in &floor_query {
        info!(?entity, "Adding cosmetics to floor {:?}", entity);
        commands.entity(entity).insert(PbrBundle {
            mesh: meshes.add(Cuboid::new(FLOOR_WIDTH, FLOOR_HEIGHT, FLOOR_LENGTH)),
            material: materials.add(Color::srgb(1.0, 1.0, 1.0)),
            ..default()
        });
    }
}

fn add_block_cosmetics(
    mut commands: Commands,
    block_query: Query<Entity, (Added<Predicted>, With<BlockMarker>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for entity in &block_query {
        info!(?entity, "Adding cosmetics to block {:?}", entity);
        commands.entity(entity).insert(PbrBundle {
            mesh: meshes.add(Cuboid::new(BLOCK_WIDTH, BLOCK_HEIGHT, BLOCK_LENGTH)),
            material: materials.add(Color::srgb(1.0, 0.0, 1.0)),
            ..default()
        });
    }
}
