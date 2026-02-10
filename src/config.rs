use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "jobtracker", about = "Job application tracker portal")]
pub struct Config {
    /// Database connection URL
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: String,

    /// Listen address
    #[arg(long, env = "LISTEN_ADDR", default_value = "0.0.0.0:8080")]
    pub listen_addr: String,

    /// Run database migrations on startup
    #[arg(long, env = "RUN_MIGRATIONS", default_value = "true")]
    pub run_migrations: bool,
}
