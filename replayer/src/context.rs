use race_env::Config;

pub struct ReplayerContext {
    pub config: Config,
}

impl ReplayerContext {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}
