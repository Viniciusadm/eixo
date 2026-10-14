use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::dto::timeline::{
    trace_collection, DayDetailResponse, DaySummaryResponse, GeocodeResponse, OverviewResponse,
    PlaceResponse, StatsResponse,
};
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::timeline::{RangeQuery, ResolvePlaceNames, TimelineQueries};
use crate::domain::{LocalDate, TravelMode};
use crate::errors::AppError;
use crate::infrastructure::geocoding::{CachedPlaceNames, NominatimResolver};
use crate::infrastructure::repositories::sqlite_timeline::SqliteTimelineRepository;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/timeline/overview", get(overview))
        .route("/timeline/days", get(days))
        .route("/timeline/days/:date", get(day))
        .route("/timeline/path", get(path))
        .route("/timeline/places", get(places))
        .route("/timeline/stats", get(stats))
        .route("/timeline/places/resolve-names", post(resolve_names))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RangeParams {
    from: Option<String>,
    to: Option<String>,
    modes: Option<String>,
}

impl TryFrom<RangeParams> for RangeQuery {
    type Error = AppError;

    fn try_from(value: RangeParams) -> Result<Self, Self::Error> {
        Ok(Self {
            from: parse_date(value.from.as_deref())?,
            to: parse_date(value.to.as_deref())?,
            modes: value
                .modes
                .as_deref()
                .map(|modes| {
                    modes
                        .split(',')
                        .map(str::trim)
                        .filter(|mode| !mode.is_empty())
                        .map(TravelMode::from_google)
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlacesParams {
    limit: Option<u32>,
}

fn parse_date(value: Option<&str>) -> Result<Option<LocalDate>, AppError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| LocalDate::parse(value).map_err(AppError::from))
        .transpose()
}

fn queries(state: &AppState) -> Result<TimelineQueries<SqliteTimelineRepository>, AppError> {
    Ok(TimelineQueries::new(SqliteTimelineRepository::new(
        state.db()?,
    )))
}

async fn overview(State(state): State<AppState>) -> Response {
    let use_cases = match queries(&state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };

    match use_cases.overview().await {
        Ok(overview) => Json(OverviewResponse::from(overview)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn days(State(state): State<AppState>, Query(params): Query<RangeParams>) -> Response {
    let (use_cases, query) = match (queries(&state), RangeQuery::try_from(params)) {
        (Ok(use_cases), Ok(query)) => (use_cases, query),
        (Err(error), _) | (_, Err(error)) => return app_error(error),
    };

    match use_cases.days(query).await {
        Ok(days) => Json(
            days.into_iter()
                .map(DaySummaryResponse::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn day(State(state): State<AppState>, Path(date): Path<String>) -> Response {
    let use_cases = match queries(&state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    let date = match LocalDate::parse(&date) {
        Ok(date) => date,
        Err(error) => return app_error(AppError::from(error)),
    };

    match use_cases.day(date).await {
        Ok(detail) => Json(DayDetailResponse::from(detail)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn path(State(state): State<AppState>, Query(params): Query<RangeParams>) -> Response {
    let (use_cases, query) = match (queries(&state), RangeQuery::try_from(params)) {
        (Ok(use_cases), Ok(query)) => (use_cases, query),
        (Err(error), _) | (_, Err(error)) => return app_error(error),
    };

    match use_cases.trace(query).await {
        Ok(lines) => Json(trace_collection(&lines)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn places(State(state): State<AppState>, Query(params): Query<PlacesParams>) -> Response {
    let use_cases = match queries(&state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };

    match use_cases.places(params.limit.unwrap_or(50)).await {
        Ok(places) => Json(
            places
                .into_iter()
                .map(PlaceResponse::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn stats(State(state): State<AppState>, Query(params): Query<RangeParams>) -> Response {
    let (use_cases, query) = match (queries(&state), RangeQuery::try_from(params)) {
        (Ok(use_cases), Ok(query)) => (use_cases, query),
        (Err(error), _) | (_, Err(error)) => return app_error(error),
    };

    match use_cases.stats(query).await {
        Ok(stats) => Json(StatsResponse::from(stats)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn resolve_names(State(state): State<AppState>) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let resolver = match NominatimResolver::new(
        &state.config.nominatim_base_url,
        &state.config.nominatim_user_agent,
    ) {
        Ok(resolver) => resolver,
        Err(error) => return app_error(error),
    };

    let use_cases = ResolvePlaceNames::new(
        SqliteTimelineRepository::new(db),
        CachedPlaceNames::load(resolver, &state.config.places_cache_path).await,
    );

    match use_cases.execute().await {
        Ok(report) => Json(GeocodeResponse::from(report)).into_response(),
        Err(error) => app_error(error),
    }
}
