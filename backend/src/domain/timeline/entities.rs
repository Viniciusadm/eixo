use crate::domain::{Coordinate, DomainId, Instant, LocalDate, PlaceKind, TimeRange, TravelMode};

#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub id: DomainId,
    pub google_place_id: String,
    pub coordinate: Coordinate,
    pub kind: PlaceKind,
    pub display_name: Option<String>,
}

impl Place {
    pub fn label(&self) -> String {
        self.display_name.clone().unwrap_or_else(|| {
            format!(
                "{:.5}, {:.5}",
                self.coordinate.latitude(),
                self.coordinate.longitude()
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Visit {
    pub id: DomainId,
    pub place_id: DomainId,
    pub range: TimeRange,
    pub local_date: LocalDate,
    pub probability: f64,
}

impl Visit {
    pub fn duration_seconds(&self) -> i64 {
        self.range.duration_seconds()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Activity {
    pub id: DomainId,
    pub range: TimeRange,
    pub local_date: LocalDate,
    pub start: Coordinate,
    pub end: Coordinate,
    pub distance_meters: f64,
    pub mode: TravelMode,
    pub probability: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TracePoint {
    pub at: Instant,
    pub coordinate: Coordinate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathSegment {
    pub id: DomainId,
    pub range: TimeRange,
    pub local_date: LocalDate,
    pub points: Vec<TracePoint>,
}

impl PathSegment {
    pub fn traced_meters(&self) -> f64 {
        self.points
            .windows(2)
            .map(|pair| pair[0].coordinate.distance_meters(&pair[1].coordinate))
            .sum()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisitedPlace {
    pub place: Place,
    pub visit_count: u32,
    pub total_seconds: i64,
    pub first_seen: Instant,
    pub last_seen: Instant,
}
