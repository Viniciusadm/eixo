use timeline_api::{api, config::AppConfig, database};
use tokio::net::TcpListener;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let dotenv_result = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    match dotenv_result {
        Ok(path) => info!("loaded environment from {}", path.display()),
        Err(err) if err.not_found() => {}
        Err(err) => warn!("failed to load .env file: {err}"),
    }

    let config = AppConfig::from_env();
    let db = if let Some(database_url) = &config.database_url {
        let pool = database::pool::connect(database_url).await?;
        if config.run_migrations {
            info!("running database migrations");
            sqlx::migrate!("./migrations").run(&pool).await?;
        }
        Some(pool)
    } else {
        warn!("DATABASE_URL not configured; API endpoints that require persistence will return configuration errors");
        None
    };

    let addr = config.http_addr();
    let app = api::router(api::state::AppState::new(db, config));
    let listener = TcpListener::bind(addr).await?;

    info!("backend listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
