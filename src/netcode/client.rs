use avian3d::prelude::SpatialQuery;
use bevy::color::Color;
use bevy::log::info;
use bevy::prelude::default;
use bevy::prelude::not;
use bevy::prelude::Added;
use bevy::prelude::Entity;
use bevy::prelude::Has;
use bevy::prelude::IntoSystemConfigs;
use bevy::prelude::KeyCode;
use bevy::prelude::TextBundle;
use bevy::text::TextStyle;
use bevy::{
    app::{FixedUpdate, Plugin, PreUpdate, Startup, Update},
    prelude::{Commands, EventReader, Query, Res, With},
    time::Time,
};
use leafwing_input_manager::prelude::ActionState;
use leafwing_input_manager::prelude::InputMap;
use leafwing_input_manager::prelude::KeyboardVirtualDPad;
use lightyear::client::events::ConnectEvent;
use lightyear::prelude::client::ClientConnection;
use lightyear::prelude::client::NetClient;
use lightyear::prelude::Replicated;
use lightyear::shared::replication::components::Controlled;
use lightyear::{
    inputs::leafwing::input_buffer::InputBuffer,
    prelude::{
        client::{ClientCommands, Predicted, PredictionSet, Rollback},
        is_host_server, MainSet, TickManager,
    },
};

use crate::netcode::protocol::*;
use crate::netcode::shared::*;

pub struct ZinnobreIronClientPlugin;

impl Plugin for ZinnobreIronClientPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Startup, connect_to_server);
        app.add_systems(
            PreUpdate,
            handle_connection
                .after(MainSet::Receive)
                .before(PredictionSet::SpawnPrediction),
        );
        app.add_systems(
            FixedUpdate,
            handle_character_actions
                .run_if(not(is_host_server))
                .in_set(FixedSet::Main),
        );
        app.add_systems(
            Update,
            (handle_new_floor, handle_new_block, handle_new_character),
        );
    }
}

fn handle_character_actions(
    time: Res<Time>,
    spatial_query: SpatialQuery,
    mut query: Query<
        (
            &ActionState<CharacterAction>,
            &InputBuffer<CharacterAction>,
            CharacterQuery,
        ),
        With<Predicted>,
    >,
    tick_manager: Res<TickManager>,
    rollback: Option<Res<Rollback>>,
) {
    let tick = rollback
        .as_ref()
        .map(|rb| tick_manager.tick_or_rollback_tick(rb))
        .unwrap_or(tick_manager.tick());

    for (action_state, input_buffer, mut character) in &mut query {
        if input_buffer.get(tick).is_some() {
            apply_character_action(&time, &spatial_query, action_state, &mut character);
            continue;
        }

        if let Some((_, prev_action_state)) = input_buffer.get_last_with_tick() {
            apply_character_action(&time, &spatial_query, prev_action_state, &mut character);
        } else {
            apply_character_action(&time, &spatial_query, action_state, &mut character);
        }
    }
}

pub(crate) fn connect_to_server(mut commands: Commands) {
    commands.connect_client();
}

pub(crate) fn handle_connection(
    mut commands: Commands,
    mut connection_event: EventReader<ConnectEvent>,
) {
    for event in connection_event.read() {
        let client_id = event.client_id();
        commands.spawn(TextBundle::from_section(
            format!("Client {}", client_id),
            TextStyle {
                font_size: 30.0,
                color: Color::WHITE,
                ..default()
            },
        ));
    }
}

fn handle_new_character(
    connection: Res<ClientConnection>,
    mut commands: Commands,
    mut character_query: Query<
        (Entity, &ColorComponent, Has<Controlled>),
        (Added<Predicted>, With<CharacterMarker>),
    >,
) {
    for (entity, color, is_controlled) in &mut character_query {
        if is_controlled {
            info!("Adding InputMap to controlled and predicted entity {entity:?}");
            // TODO: refactor to input module
            commands.entity(entity).insert(
                InputMap::new([(CharacterAction::Jump, KeyCode::Space)])
                    .with_dual_axis(CharacterAction::Move, KeyboardVirtualDPad::WASD),
            );
        } else {
            info!("Remote character replicated to us: {entity:?}");
        }
        let client_id = connection.id();
        info!(?entity, ?client_id, "Adding physics to character");
        commands
            .entity(entity)
            .insert((CharacterPhysicsBundle::default(),));
    }
}

fn handle_new_floor(
    _connection: Res<ClientConnection>,
    mut commands: Commands,
    character_query: Query<Entity, (Added<Replicated>, With<FloorMarker>)>,
) {
    for entity in &character_query {
        info!(?entity, "Adding physics to floor");
        commands
            .entity(entity)
            .insert(FloorPhysicsBundle::default());
    }
}

fn handle_new_block(
    _connection: Res<ClientConnection>,
    mut commands: Commands,
    character_query: Query<Entity, (Added<Predicted>, With<BlockMarker>)>,
) {
    for entity in &character_query {
        info!(?entity, "Adding physics to block");
        commands
            .entity(entity)
            .insert(BlockPhysicsBundle::default());
    }
}
