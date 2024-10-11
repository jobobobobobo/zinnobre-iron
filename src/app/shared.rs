use lightyear::prelude::{Mode, SharedConfig, TickConfig};
use std::time::Duration;

pub const FIXED_TIMESTEP_HZ: f64 = 64.0;

pub const REPLICATION_INTERVAL: Duration = Duration::from_millis(100);

pub fn shared_config(mode: Mode) -> SharedConfig {
    SharedConfig {
        server_replication_send_interval: REPLICATION_INTERVAL,
        tick: TickConfig {
            tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
        },
        mode,
    }
}
