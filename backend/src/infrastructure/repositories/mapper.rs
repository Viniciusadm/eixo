use crate::domain::{Coordinate, DomainId, Instant, LocalDate, PlaceKind, TimeRange, TravelMode};
use crate::errors::AppError;

pub fn id(value: String) -> Result<DomainId, AppError> {
    Ok(DomainId::new(value)?)
}

pub fn coordinate(latitude: f64, longitude: f64) -> Result<Coordinate, AppError> {
    Ok(Coordinate::new(latitude, longitude)?)
}

pub fn instant(value: &str, offset_minutes: i64) -> Result<Instant, AppError> {
    let parsed = Instant::from_rfc3339(value)?;
    Ok(Instant::from_utc(parsed.utc(), offset_minutes as i32))
}

pub fn time_range(
    started_at: &str,
    ended_at: &str,
    offset_minutes: i64,
) -> Result<TimeRange, AppError> {
    Ok(TimeRange::new(
        instant(started_at, offset_minutes)?,
        instant(ended_at, offset_minutes)?,
    )?)
}

pub fn local_date(value: &str) -> Result<LocalDate, AppError> {
    Ok(LocalDate::parse(value)?)
}

pub fn travel_mode(value: &str) -> TravelMode {
    TravelMode::from_google(value)
}

pub fn place_kind(value: &str) -> PlaceKind {
    PlaceKind::from_google(value)
}
