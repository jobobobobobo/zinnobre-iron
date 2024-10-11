mod settings;
mod shared;

use crate::app::server::plugin::ServerPlugins;
use crate::app::settings::{build_client_netcode_config, Settings};
use bevy::log::{Level, LogPlugin};
use bevy::prelude::App;
use bevy::prelude::AssetPlugin;
use bevy::prelude::Plugin;
use bevy::state::app::StatesPlugin;
use bevy::utils::default;
use bevy::DefaultPlugins;
use bevy::MinimalPlugins;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use clap::Parser;
use lightyear::client::plugin::ClientPlugins;
use lightyear::prelude::client::ClientTransport;
use lightyear::prelude::server::ServerTransport;
use lightyear::prelude::ReplicationConfig;
use lightyear::shared::config::Mode;
use lightyear::{
    client::config::ClientConfig,
    connection::client,
    server::{self, config::ServerConfig},
    transport::LOCAL_SOCKET,
};
use settings::{build_server_netcode_config, get_client_net_config, get_server_net_configs};
use shared::{shared_config, REPLICATION_INTERVAL};
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Parser, PartialEq, Debug)]
pub enum Cli {
    HostServer {
        #[arg(short, long, default_value = None)]
        client_id: Option<u64>,
    },
    ClientAndServer {
        #[arg(short, long, default_value = None)]
        client_id: Option<u64>,
    },
    Server,
    Client {
        #[arg(short, long, default_value = None)]
        client_id: Option<u64>,
    },
}

struct SendApp(App);

unsafe impl Send for SendApp {}
impl SendApp {
    fn run(&mut self) {
        self.0.run();
    }
}

impl Default for Cli {
    fn default() -> Self {
        cli()
    }
}

pub fn cli() -> Cli {
    Cli::parse()
}

pub enum Apps {
    Client {
        app: App,
        config: ClientConfig,
    },
    Server {
        app: App,
        config: ServerConfig,
    },
    ClientAndServer {
        client_app: App,
        client_config: ClientConfig,
        server_app: App,
        server_config: ServerConfig,
    },
    HostServer {
        app: App,
        client_config: ClientConfig,
        server_config: ServerConfig,
    },
}

impl Apps {
    pub fn new(settings: Settings, cli: Cli) -> Self {
        match cli {
            Cli::HostServer { client_id } => {
                let client_net_config = client::NetConfig::Local {
                    id: client_id.unwrap_or(settings.client.client_id),
                };
                let (app, client_config, server_config) =
                    combined_app(settings, vec![], client_net_config);
                Apps::HostServer {
                    app,
                    client_config,
                    server_config,
                }
            }
            Cli::ClientAndServer { client_id } => {
                let (from_server_send, from_server_recv) = crossbeam_channel::unbounded();
                let (to_server_send, to_server_recv) = crossbeam_channel::unbounded();
                let transport_config = ClientTransport::LocalChannel {
                    recv: from_server_recv,
                    send: to_server_send,
                };

                let net_config = build_client_netcode_config(
                    client_id.unwrap_or(settings.client.client_id),
                    LOCAL_SOCKET,
                    settings.client.conditioner.as_ref(),
                    &settings.shared,
                    transport_config,
                );
                let (client_app, client_config) = client_app(settings.clone(), net_config);

                let extra_transport_configs = vec![ServerTransport::Channels {
                    channels: vec![(LOCAL_SOCKET, to_server_recv, from_server_send)],
                }];
                let (server_app, server_config) = server_app(settings, extra_transport_configs);
                Apps::ClientAndServer {
                    client_app,
                    client_config,
                    server_app,
                    server_config,
                }
            }
            Cli::Server => {
                let (app, config) = server_app(settings, vec![]);
                Apps::Server { app, config }
            }
            Cli::Client { client_id } => {
                let server_addr = SocketAddr::new(
                    settings.client.server_addr.into(),
                    settings.client.server_port,
                );
                let client_id = client_id.unwrap_or(settings.client.client_id);
                let net_config = get_client_net_config(&settings, client_id);
                let (app, config) = client_app(settings, net_config);
                Apps::Client { app, config }
            }
        }
    }
    pub fn with_server_replication_send_interval(mut self, replication_interval: Duration) -> Self {
        self.update_lightyear_client_config(|cc: &mut ClientConfig| {
            cc.shared.server_replication_send_interval = replication_interval;
        });
        self.update_lightyear_server_config(|sc: &mut ServerConfig| {
            sc.shared.server_replication_send_interval = replication_interval;
            sc.replication.send_interval = replication_interval;
        });
        self
    }

    pub fn add_lightyear_plugins(&mut self) -> &mut Self {
        match self {
            Apps::Client { app, config } => {
                app.add_plugins(ClientPlugins {
                    config: config.clone(),
                });
            }
            Apps::Server { app, config } => {
                app.add_plugins(ServerPlugins {
                    config: config.clone(),
                });
            }
            Apps::ClientAndServer {
                client_app,
                client_config,
                server_app,
                server_config,
            } => {
                client_app.add_plugins(ClientPlugins {
                    config: client_config.clone(),
                });
                server_app.add_plugins(ServerPlugins {
                    config: server_config.clone(),
                });
            }
            Apps::HostServer {
                app,
                client_config,
                server_config,
            } => {
                // TODO: currently, need serverplugins to run first because it adds the shared plugins
                // not super relevant but not ideal
                app.add_plugins(ClientPlugins {
                    config: client_config.clone(),
                });
                app.add_plugins(ServerPlugins {
                    config: server_config.clone(),
                });
            }
        }
        self
    }

    pub fn add_user_plugins(
        &mut self,
        client_plugin: impl Plugin,
        server_plugin: impl Plugin,
        shared_plugin: impl Plugin + Clone,
    ) -> &mut Self {
        match self {
            Apps::Client { app, .. } => {
                app.add_plugins((client_plugin, shared_plugin));
            }
            Apps::Server { app, .. } => {
                app.add_plugins((server_plugin, shared_plugin));
            }
            Apps::ClientAndServer {
                client_app,
                server_app,
                ..
            } => {
                client_app.add_plugins((client_plugin, shared_plugin.clone()));
                server_app.add_plugins((server_plugin, shared_plugin));
            }
            Apps::HostServer { app, .. } => {
                app.add_plugins((client_plugin, server_plugin, shared_plugin));
            }
        }
        self
    }

    pub fn update_lightyear_client_config(
        &mut self,
        f: impl FnOnce(&mut ClientConfig),
    ) -> &mut Self {
        match self {
            Apps::Client { config, .. } => {
                f(config);
            }
            Apps::Server { config, .. } => {}
            Apps::ClientAndServer { client_config, .. } => {
                f(client_config);
            }
            Apps::HostServer { client_config, .. } => {
                f(client_config);
            }
        }
        self
    }

    pub fn update_lightyear_server_config(
        &mut self,
        f: impl FnOnce(&mut ServerConfig),
    ) -> &mut Self {
        match self {
            Apps::Client { config, .. } => {}
            Apps::Server { config, .. } => {
                f(config);
            }
            Apps::ClientAndServer { server_config, .. } => {
                f(server_config);
            }
            Apps::HostServer { server_config, .. } => {
                f(server_config);
            }
        }
        self
    }

    pub fn run(self) {
        match self {
            Apps::Client { mut app, .. } => {
                app.run();
            }
            Apps::Server { mut app, .. } => {
                app.run();
            }
            Apps::ClientAndServer {
                mut client_app,
                server_app,
                ..
            } => {
                let mut send_app = SendApp(server_app);
                std::thread::spawn(move || send_app.run());
                client_app.run();
            }
            Apps::HostServer { mut app, .. } => {
                app.run();
            }
        }
    }
}

fn client_app(settings: Settings, net_config: client::NetConfig) -> (App, ClientConfig) {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .build()
            .set(AssetPlugin {
                // https://github.com/bevyengine/bevy/issues/10157
                meta_check: bevy::asset::AssetMetaCheck::Never,
                ..default()
            })
            .set(LogPlugin {
                level: Level::INFO,
                filter: "wgpu=error,bevy_render=info,bevy_ecs=warn".to_string(),
                ..default()
            }),
    );
    if settings.client.inspector {
        app.add_plugins(WorldInspectorPlugin::new());
    }
    let client_config = ClientConfig {
        shared: shared_config(Mode::Separate),
        net: net_config,
        replication: ReplicationConfig {
            send_interval: REPLICATION_INTERVAL,
            ..default()
        },
        ..default()
    };
    (app, client_config)
}

fn server_app(
    settings: Settings,
    extra_transport_configs: Vec<ServerTransport>,
) -> (App, ServerConfig) {
    let mut app = App::new();
    if !settings.server.headless {
        app.add_plugins(DefaultPlugins.build().disable::<LogPlugin>());
    } else {
        app.add_plugins((MinimalPlugins, StatesPlugin));
    }
    app.add_plugins(LogPlugin {
        level: Level::INFO,
        filter: "wgpu=error,bevy_render=info,bevy_ecs=warn".to_string(),
        ..default()
    });

    if settings.server.inspector {
        app.add_plugins(WorldInspectorPlugin::new());
    }

    let mut net_configs = get_server_net_configs(&settings);
    let extra_net_configs = extra_transport_configs.into_iter().map(|c| {
        build_server_netcode_config(settings.server.conditioner.as_ref(), &settings.shared, c)
    });
    net_configs.extend(extra_net_configs);
    let server_config = ServerConfig {
        shared: shared_config(Mode::Separate),
        net: net_configs,
        replication: ReplicationConfig {
            send_interval: REPLICATION_INTERVAL,
            ..default()
        },
        ..default()
    };
    (app, server_config)
}

fn combined_app(
    settings: Settings,
    extra_transport_configs: Vec<ServerTransport>,
    client_net_config: client::NetConfig,
) -> (App, ClientConfig, ServerConfig) {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.build().set(LogPlugin {
        level: Level::INFO,
        filter: "wgpu=error,bevy_render=info,bevy_ecs=warn".to_string(),
        ..default()
    }));
    if settings.client.inspector {
        app.add_plugins(WorldInspectorPlugin::new());
    }
    let mut net_configs = get_server_net_configs(&settings);
    let extra_net_configs = extra_transport_configs.into_iter().map(|c| {
        build_server_netcode_config(settings.server.conditioner.as_ref(), &settings.shared, c)
    });
    net_configs.extend(extra_net_configs);
    let client_config = ClientConfig {
        shared: shared_config(Mode::HostServer),
        net: client_net_config,
        ..default()
    };
    let server_config = ServerConfig {
        shared: shared_config(Mode::HostServer),
        net: net_configs,
        ..default()
    };
    (app, client_config, server_config)
}
