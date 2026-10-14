use crate::domain::{Coordinate, Instant, LocalDate, PlaceKind, TimeRange, TravelMode};

#[derive(Debug, Clone, PartialEq)]
pub struct RangeQuery {
    pub from: Option<LocalDate>,
    pub to: Option<LocalDate>,
    pub modes: Vec<TravelMode>,
}

impl RangeQuery {
    pub fn unbounded() -> Self {
        Self {
            from: None,
            to: None,
            modes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineOverview {
    pub first_date: Option<LocalDate>,
    pub last_date: Option<LocalDate>,
    pub tracked_days: u32,
    pub total_distance_meters: f64,
    pub visit_count: u32,
    pub place_count: u32,
    pub trace_point_count: u32,
    pub modes: Vec<ModeTotal>,
    pub daily_distance: Vec<DailyDistance>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModeTotal {
    pub mode: TravelMode,
    pub trip_count: u32,
    pub distance_meters: f64,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DailyDistance {
    pub date: LocalDate,
    pub distance_meters: f64,
    pub trip_count: u32,
    pub visit_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DaySummary {
    pub date: LocalDate,
    pub distance_meters: f64,
    pub moving_seconds: i64,
    pub trip_count: u32,
    pub visit_count: u32,
    pub modes: Vec<TravelMode>,
    pub first_activity_at: Option<Instant>,
    pub last_activity_at: Option<Instant>,
    pub highlight_place: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DayDetail {
    pub date: LocalDate,
    pub summary: DaySummary,
    pub entries: Vec<DayEntry>,
    pub trace: Vec<TraceLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DayEntry {
    Stay(StayEntry),
    Journey(JourneyEntry),
}

impl DayEntry {
    pub fn started_at(&self) -> Instant {
        match self {
            Self::Stay(entry) => entry.range.start(),
            Self::Journey(entry) => entry.range.start(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StayEntry {
    pub id: String,
    pub place_id: String,
    pub label: String,
    pub kind: PlaceKind,
    pub coordinate: Coordinate,
    pub range: TimeRange,
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JourneyEntry {
    pub id: String,
    pub mode: TravelMode,
    pub range: TimeRange,
    pub start: Coordinate,
    pub end: Coordinate,
    pub distance_meters: f64,
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraceLine {
    pub id: String,
    pub date: LocalDate,
    pub mode: Option<TravelMode>,
    pub points: Vec<TracePointView>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TracePointView {
    pub at: Instant,
    pub coordinate: Coordinate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlaceRanking {
    pub place_id: String,
    pub label: String,
    pub kind: PlaceKind,
    pub coordinate: Coordinate,
    pub visit_count: u32,
    pub total_seconds: i64,
    pub first_seen: Instant,
    pub last_seen: Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RangeStats {
    pub from: Option<LocalDate>,
    pub to: Option<LocalDate>,
    pub distance_meters: f64,
    pub moving_seconds: i64,
    pub tracked_days: u32,
    pub trip_count: u32,
    pub visit_count: u32,
    pub distinct_places: u32,
    pub modes: Vec<ModeTotal>,
    pub busiest_day: Option<DailyDistance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportReport {
    pub segments: u32,
    pub places: u32,
    pub visits: u32,
    pub activities: u32,
    pub path_segments: u32,
    pub trace_points: u32,
    pub skipped: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GeocodeReport {
    pub pending: u32,
    pub resolved: u32,
    pub failed: u32,
}
