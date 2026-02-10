use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "jobtracker", about = "Job application tracker portal")]
pub struct Config {
    /// Database connection URL
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: String,

    /// Run database migrations on startup
    #[arg(long, env = "RUN_MIGRATIONS", default_value = "true")]
    pub run_migrations: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum Command {
    /// Start the web server (default when no subcommand given)
    Serve {
        /// Listen address
        #[arg(long, env = "LISTEN_ADDR", default_value = "0.0.0.0:8080")]
        listen_addr: String,
    },
    /// Run a job collector worker loop
    Collect {
        /// Collector name (must match a row in the collectors table)
        #[arg(long)]
        collector: String,

        /// Poll interval in seconds
        #[arg(long, env = "POLL_INTERVAL", default_value = "10")]
        poll_interval: u64,
    },
}

impl Config {
    /// Resolve the command, defaulting to Serve if none specified.
    pub fn resolved_command(&self) -> Command {
        self.command.clone().unwrap_or(Command::Serve {
            listen_addr: std::env::var("LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
        })
    }
}
