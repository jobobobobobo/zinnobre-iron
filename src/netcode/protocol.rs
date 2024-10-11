use avian3d::prelude::AngularVelocity;
use avian3d::prelude::LinearVelocity;
use avian3d::prelude::Position;
use avian3d::prelude::Rotation;
use bevy::app::App;
use bevy::app::Plugin;
use bevy::core::Name;
use bevy::prelude::Color;
use bevy::prelude::Component;
use bevy::prelude::Reflect;
use leafwing_input_manager::Actionlike;
use lightyear::channel::builder::ChannelDirection;
use lightyear::client::components::ComponentSyncMode;
use lightyear::prelude::AppComponentExt;
use lightyear::prelude::LeafwingInputPlugin;
use lightyear::prelude::ReplicationGroup;
use lightyear::utils::avian3d::position;
use lightyear::utils::avian3d::rotation;
use serde::Deserialize;
use serde::Serialize;

pub const REPLICATION_GROUP: ReplicationGroup = ReplicationGroup::new_id(1);

#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct ColorComponent(pub(crate) Color);

#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct CharacterMarker;

#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct FloorMarker;

#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct BlockMarker;

#[derive(Copy, Deserialize, Serialize, Clone, Debug, PartialEq, Reflect, Hash, Eq)]
pub enum CharacterAction {
    Move,
    Jump,
}

impl Actionlike for CharacterAction {
    fn input_control_kind(&self) -> leafwing_input_manager::InputControlKind {
        match self {
            Self::Move => leafwing_input_manager::InputControlKind::DualAxis,
            Self::Jump => leafwing_input_manager::InputControlKind::Button,
        }
    }
}

pub(crate) struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LeafwingInputPlugin::<CharacterAction>::default());

        app.register_component::<ColorComponent>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Once);

        app.register_component::<Name>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Once);

        app.register_component::<CharacterMarker>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Once);

        app.register_component::<FloorMarker>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Once);

        app.register_component::<BlockMarker>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Once);

        app.register_component::<LinearVelocity>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        app.register_component::<AngularVelocity>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        app.register_component::<Position>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full)
            .add_interpolation_fn(position::lerp)
            .add_correction_fn(position::lerp);

        app.register_component::<Rotation>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full)
            .add_interpolation_fn(rotation::lerp)
            .add_correction_fn(rotation::lerp);
    }
}
