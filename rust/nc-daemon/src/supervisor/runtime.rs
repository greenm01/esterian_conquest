use crate::config::daemon_config::DaemonConfig;

#[derive(Debug)]
pub struct SupervisorRuntime {
    pub config: DaemonConfig,
}
