use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub host: IpAddr,
    pub port: u16,
    pub database_url: Option<String>,
    pub run_migrations: bool,
    pub timeline_json_path: PathBuf,
    pub nominatim_base_url: String,
    pub nominatim_user_agent: String,
    pub places_cache_path: PathBuf,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let host = env::var("HTTP_HOST")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        let port = env::var("HTTP_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3001);
        let database_url = env::var("DATABASE_URL")
            .ok()
            .or_else(|| Some("sqlite://data/timeline.db".to_owned()));
        let run_migrations = env::var("RUN_MIGRATIONS")
            .map(|value| !matches!(value.as_str(), "0" | "false" | "FALSE" | "no" | "NO"))
            .unwrap_or(true);

        Self {
            host,
            port,
            database_url,
            run_migrations,
            timeline_json_path: env::var("TIMELINE_JSON_PATH")
                .unwrap_or_else(|_| "../timeline.json".to_owned())
                .into(),
            nominatim_base_url: env::var("NOMINATIM_BASE_URL")
                .unwrap_or_else(|_| "https://nominatim.openstreetmap.org".to_owned()),
            nominatim_user_agent: env::var("NOMINATIM_USER_AGENT")
                .unwrap_or_else(|_| "timeline-api/0.1 (self-hosted)".to_owned()),
            places_cache_path: env::var("PLACES_CACHE_PATH")
                .unwrap_or_else(|_| "data/places-cache.json".to_owned())
                .into(),
        }
    }

    pub fn http_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }
}
