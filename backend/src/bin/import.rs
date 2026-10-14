use std::env;

use timeline_api::application::timeline::{ImportTimeline, ResolvePlaceNames};
use timeline_api::config::AppConfig;
use timeline_api::database;
use timeline_api::infrastructure::geocoding::{CachedPlaceNames, NominatimResolver};
use timeline_api::infrastructure::repositories::sqlite_timeline::SqliteTimelineRepository;
use timeline_api::infrastructure::timeline::google_takeout::GoogleTakeoutSource;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = AppConfig::from_env();
    let arguments: Vec<String> = env::args().skip(1).collect();
    let geocode = arguments.iter().any(|argument| argument == "--geocode");
    let source_path = arguments
        .iter()
        .find(|argument| !argument.starts_with("--"))
        .cloned()
        .map(Into::into)
        .unwrap_or_else(|| config.timeline_json_path.clone());

    let database_url = config
        .database_url
        .as_deref()
        .ok_or("DATABASE_URL is not configured")?;
    let pool = database::pool::connect(database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let repository = SqliteTimelineRepository::new(pool);
    let report = ImportTimeline::new(GoogleTakeoutSource::new(&source_path), repository.clone())
        .execute()
        .await?;

    println!(
        "imported {} segments from {}\n  places        {}\n  visits        {}\n  activities    {}\n  path segments {}\n  trace points  {}\n  skipped       {}",
        report.segments,
        source_path.display(),
        report.places,
        report.visits,
        report.activities,
        report.path_segments,
        report.trace_points,
        report.skipped,
    );

    if geocode {
        println!("\nresolving place names via {}", config.nominatim_base_url);
        let resolver = CachedPlaceNames::load(
            NominatimResolver::new(&config.nominatim_base_url, &config.nominatim_user_agent)?,
            &config.places_cache_path,
        )
        .await;
        let geocode_report = ResolvePlaceNames::new(repository, resolver)
            .execute()
            .await?;
        println!(
            "  pending  {}\n  resolved {}\n  failed   {}",
            geocode_report.pending, geocode_report.resolved, geocode_report.failed
        );
    }

    Ok(())
}
