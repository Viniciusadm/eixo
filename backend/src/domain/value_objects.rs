use std::fmt;

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, Utc};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DomainId(String);

impl DomainId {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainValidationError::Empty("id"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    pub fn new(
        value: impl Into<String>,
        field: &'static str,
    ) -> Result<Self, DomainValidationError> {
        let value = value.into().trim().to_owned();
        if value.is_empty() {
            return Err(DomainValidationError::Empty(field));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coordinate {
    latitude: f64,
    longitude: f64,
}

impl Coordinate {
    pub fn new(latitude: f64, longitude: f64) -> Result<Self, DomainValidationError> {
        if !latitude.is_finite() || !(-90.0..=90.0).contains(&latitude) {
            return Err(DomainValidationError::Invalid("latitude"));
        }
        if !longitude.is_finite() || !(-180.0..=180.0).contains(&longitude) {
            return Err(DomainValidationError::Invalid("longitude"));
        }
        Ok(Self {
            latitude,
            longitude,
        })
    }

    pub fn latitude(&self) -> f64 {
        self.latitude
    }

    pub fn longitude(&self) -> f64 {
        self.longitude
    }

    pub fn distance_meters(&self, other: &Self) -> f64 {
        const EARTH_RADIUS_METERS: f64 = 6_371_008.8;
        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let delta_lat = (other.latitude - self.latitude).to_radians();
        let delta_lng = (other.longitude - self.longitude).to_radians();
        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (delta_lng / 2.0).sin().powi(2);
        2.0 * EARTH_RADIUS_METERS * a.sqrt().asin()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant {
    utc: DateTime<Utc>,
    offset_minutes: i32,
}

impl Instant {
    pub fn from_rfc3339(value: &str) -> Result<Self, DomainValidationError> {
        let parsed = DateTime::parse_from_rfc3339(value.trim())
            .map_err(|_| DomainValidationError::Invalid("timestamp"))?;
        Ok(Self {
            utc: parsed.with_timezone(&Utc),
            offset_minutes: parsed.offset().local_minus_utc() / 60,
        })
    }

    pub fn from_utc(utc: DateTime<Utc>, offset_minutes: i32) -> Self {
        Self {
            utc,
            offset_minutes,
        }
    }

    pub fn utc(&self) -> DateTime<Utc> {
        self.utc
    }

    pub fn offset_minutes(&self) -> i32 {
        self.offset_minutes
    }

    pub fn local(&self) -> DateTime<FixedOffset> {
        let offset = FixedOffset::east_opt(self.offset_minutes * 60)
            .unwrap_or_else(|| FixedOffset::east_opt(0).expect("zero offset is valid"));
        self.utc.with_timezone(&offset)
    }

    pub fn local_date(&self) -> LocalDate {
        LocalDate(self.local().date_naive())
    }

    pub fn to_storage(&self) -> String {
        self.utc.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
    }

    pub fn to_rfc3339(&self) -> String {
        self.local().to_rfc3339()
    }

    pub fn seconds_until(&self, other: &Self) -> i64 {
        (other.utc - self.utc).num_seconds()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalDate(NaiveDate);

impl LocalDate {
    pub fn parse(value: &str) -> Result<Self, DomainValidationError> {
        NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
            .map(Self)
            .map_err(|_| DomainValidationError::Invalid("local_date"))
    }

    pub fn as_naive(&self) -> NaiveDate {
        self.0
    }

    pub fn to_iso(&self) -> String {
        self.0.format("%Y-%m-%d").to_string()
    }

    pub fn weekday_index(&self) -> u32 {
        self.0.weekday().num_days_from_monday()
    }

    pub fn next(&self) -> Self {
        Self(self.0 + Duration::days(1))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    start: Instant,
    end: Instant,
}

impl TimeRange {
    pub fn new(start: Instant, end: Instant) -> Result<Self, DomainValidationError> {
        if end.utc() < start.utc() {
            return Err(DomainValidationError::Invalid("time_range"));
        }
        Ok(Self { start, end })
    }

    pub fn start(&self) -> Instant {
        self.start
    }

    pub fn end(&self) -> Instant {
        self.end
    }

    pub fn duration_seconds(&self) -> i64 {
        self.start.seconds_until(&self.end)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TravelMode {
    Walking,
    PassengerVehicle,
    Subway,
    Bus,
    Cycling,
    Train,
    Taxi,
    Motorcycle,
    Ferry,
    Flying,
    Running,
    Other(String),
}

impl TravelMode {
    pub fn from_google(value: &str) -> Self {
        match value.trim().to_ascii_uppercase().as_str() {
            "WALKING" | "ON_FOOT" => Self::Walking,
            "IN_PASSENGER_VEHICLE" | "DRIVING" | "IN_VEHICLE" => Self::PassengerVehicle,
            "IN_SUBWAY" => Self::Subway,
            "IN_BUS" => Self::Bus,
            "CYCLING" | "ON_BICYCLE" => Self::Cycling,
            "IN_TRAIN" | "IN_TRAM" => Self::Train,
            "IN_TAXI" => Self::Taxi,
            "MOTORCYCLING" => Self::Motorcycle,
            "IN_FERRY" => Self::Ferry,
            "FLYING" => Self::Flying,
            "RUNNING" => Self::Running,
            other => Self::Other(other.to_owned()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Walking => "WALKING",
            Self::PassengerVehicle => "IN_PASSENGER_VEHICLE",
            Self::Subway => "IN_SUBWAY",
            Self::Bus => "IN_BUS",
            Self::Cycling => "CYCLING",
            Self::Train => "IN_TRAIN",
            Self::Taxi => "IN_TAXI",
            Self::Motorcycle => "MOTORCYCLING",
            Self::Ferry => "IN_FERRY",
            Self::Flying => "FLYING",
            Self::Running => "RUNNING",
            Self::Other(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceKind {
    Home,
    Work,
    Unknown,
}

impl PlaceKind {
    pub fn from_google(value: &str) -> Self {
        match value.trim().to_ascii_uppercase().as_str() {
            "INFERRED_HOME" | "HOME" => Self::Home,
            "INFERRED_WORK" | "WORK" => Self::Work,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Home => "HOME",
            Self::Work => "WORK",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainValidationError {
    Empty(&'static str),
    Invalid(&'static str),
}

impl fmt::Display for DomainValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty(field) => write!(f, "{field} must not be empty"),
            Self::Invalid(field) => write!(f, "{field} is not valid"),
        }
    }
}
