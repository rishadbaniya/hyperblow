use std::{env, io::stdout};
use tracing::{debug, subscriber::set_global_default};
use tracing_subscriber::EnvFilter;

pub struct StdoutLogger;

impl StdoutLogger {
    pub fn init_from_env() {
        let Ok(filter) = env::var("HYPERBLOW_LOG") else {
            return;
        };
        if filter.trim().is_empty() {
            return;
        }
        let env_filter = match EnvFilter::try_new(filter.trim()) {
            Ok(filter) => filter,
            Err(error) => {
                println!("hyperblow stdout logging disabled: invalid HYPERBLOW_LOG value: {error}");
                return;
            }
        };

        let subscriber = tracing_subscriber::fmt().with_env_filter(env_filter).with_writer(stdout).finish();

        let _ = set_global_default(subscriber);
        debug!("stdout logging initialized");
    }
}
