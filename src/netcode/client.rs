use crate::input::BasicAction;
use bevy::app::Plugin;
use leafwing_input_manager::{plugin::InputManagerPlugin, prelude::ActionState};

pub struct ZinnobreIronClientPlugin;

impl Plugin for ZinnobreIronClientPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(InputManagerPlugin::<BasicAction>::default())
            .init_resource::<ActionState<BasicAction>>()
            .insert_resource(BasicAction::mkb_input_map());
    }
}
