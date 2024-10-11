use avian3d::prelude::Position;
use avian3d::prelude::SpatialQuery;
use bevy::app::FixedUpdate;
use bevy::app::Plugin;
use bevy::app::PreUpdate;
use bevy::app::Update;
use bevy::color::palettes::css;
use bevy::core::Name;
use bevy::log::info;
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use lightyear::channel::builder::InputChannel;
use lightyear::prelude::server::ControlledBy;
use lightyear::prelude::server::Replicate;
use lightyear::prelude::server::ServerCommands;
use lightyear::prelude::server::SyncTarget;
use lightyear::prelude::InputMessage;
use lightyear::prelude::MainSet;
use lightyear::server::connection::ConnectionManager;
use lightyear::server::events::ConnectEvent;
use lightyear::server::events::MessageEvent;
use lightyear::shared::replication::network_target::NetworkTarget;

use crate::netcode::protocol::*;
use crate::netcode::shared::*;

pub struct ZinnobreIronServerPlugin;

impl Plugin for ZinnobreIronServerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(PreUpdate, replicate_inputs.after(MainSet::EmitEvents));
        app.add_systems(FixedUpdate, handle_character_actions.in_set(FixedSet::Main));
        app.add_systems(Update, handle_connections);
    }
}

fn handle_character_actions(
    time: Res<Time>,
    spatial_query: SpatialQuery,
    mut query: Query<(&ActionState<CharacterAction>, CharacterQuery)>,
) {
    for (action_state, mut character) in &mut query {
        apply_character_action(&time, &spatial_query, action_state, &mut character);
    }
}

fn init(mut commands: Commands) {
    commands.start_server();

    commands.spawn((
        Name::new("Floor"),
        FloorPhysicsBundle::default(),
        FloorMarker,
        Position::new(Vec3::ZERO),
        Replicate::default(),
    ));

    let block_replicate_component = Replicate {
        sync: SyncTarget {
            prediction: lightyear::prelude::NetworkTarget::All,
            ..default()
        },
        group: REPLICATION_GROUP,
        ..default()
    };
    commands.spawn((
        Name::new("Block"),
        BlockPhysicsBundle::default(),
        BlockMarker,
        Position::new(Vec3::new(-1.0, 1.0, 0.0)),
        block_replicate_component.clone(),
    ));
}

pub(crate) fn replicate_inputs(
    mut connection: ResMut<ConnectionManager>,
    mut input_events: ResMut<Events<MessageEvent<InputMessage<CharacterAction>>>>,
) {
    for mut event in input_events.drain() {
        let client_id = *event.context();
        connection
            .send_message_to_target::<InputChannel, _>(
                &mut event.message,
                NetworkTarget::AllExceptSingle(client_id),
            )
            .unwrap()
    }
}

pub(crate) fn handle_connections(
    mut connections: EventReader<ConnectEvent>,
    mut commands: Commands,
    character_query: Query<Entity, With<CharacterMarker>>,
) {
    let mut num_characters = character_query.iter().count();
    for connection in connections.read() {
        let client_id = connection.client_id;
        info!("Client connected with client-id {client_id:?}. Spawning character entity.");
        let replicate = Replicate {
            sync: SyncTarget {
                prediction: lightyear::prelude::NetworkTarget::All,
                ..default()
            },
            controlled_by: ControlledBy {
                target: lightyear::prelude::NetworkTarget::Single(client_id),
                ..default()
            },
            group: REPLICATION_GROUP,
            ..default()
        };

        let available_colors = [
            css::LIMEGREEN,
            css::PINK,
            css::YELLOW,
            css::AQUA,
            css::CRIMSON,
            css::GOLD,
            css::ORANGE_RED,
            css::SILVER,
            css::SALMON,
            css::YELLOW_GREEN,
            css::WHITE,
            css::RED,
        ];
        let color = available_colors[num_characters % available_colors.len()];
        let angle: f32 = num_characters as f32 * 5.0;
        let x = 2.0 * angle.cos();
        let z = 2.0 * angle.sin();

        let character = commands
            .spawn((
                Name::new("Character"),
                ActionState::<CharacterAction>::default(),
                Position(Vec3::new(x, 3.0, z)),
                replicate,
                CharacterPhysicsBundle::default(),
                ColorComponent(color.into()),
                CharacterMarker,
            ))
            .id();

        info!("Created entity {character:?} for client {client_id:?}");
        num_characters += 1;
    }
}
