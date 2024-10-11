use app::settings::{read_settings, Settings};
use app::{Apps, Cli};
use netcode::client::ZinnobreIronClientPlugin;
use netcode::server::ZinnobreIronServerPlugin;
use netcode::shared::SharedPlugin;
use serde::{Deserialize, Serialize};

mod app;
mod input;
mod netcode;
mod render;

fn main() {
    let cli = Cli::default();
    let settings_str = include_str!("../assets/settings.ron");
    let settings = read_settings::<ZinnobreIronSettings>(settings_str);
    let mut apps = Apps::new(settings.common, cli);
    apps.update_lightyear_client_config(|config| {
        config.prediction.minimum_input_delay_ticks = settings.input_delay_ticks;
        config.prediction.correction_ticks_factor = settings.correction_ticks_factor;
    })
    .add_lightyear_plugins()
    .add_user_plugins(
        ZinnobreIronClientPlugin,
        ZinnobreIronServerPlugin,
        SharedPlugin,
    );

    apps.run();
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ZinnobreIronSettings {
    pub common: Settings,

    pub(crate) input_delay_ticks: u16,

    pub(crate) correction_ticks_factor: f32,
}
