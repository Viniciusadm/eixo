use serde::Serialize;
use serde_json::{json, Value};

use crate::application::timeline::{
    DailyDistance, DayDetail, DayEntry, DaySummary, GeocodeReport, ModeTotal, PlaceRanking,
    RangeStats, TimelineOverview, TraceLine,
};
use crate::domain::Coordinate;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewResponse {
    pub first_date: Option<String>,
    pub last_date: Option<String>,
    pub tracked_days: u32,
    pub total_distance_meters: f64,
    pub visit_count: u32,
    pub place_count: u32,
    pub trace_point_count: u32,
    pub modes: Vec<ModeTotalResponse>,
    pub daily_distance: Vec<DailyDistanceResponse>,
}

impl From<TimelineOverview> for OverviewResponse {
    fn from(value: TimelineOverview) -> Self {
        Self {
            first_date: value.first_date.map(|date| date.to_iso()),
            last_date: value.last_date.map(|date| date.to_iso()),
            tracked_days: value.tracked_days,
            total_distance_meters: value.total_distance_meters,
            visit_count: value.visit_count,
            place_count: value.place_count,
            trace_point_count: value.trace_point_count,
            modes: value
                .modes
                .into_iter()
                .map(ModeTotalResponse::from)
                .collect(),
            daily_distance: value
                .daily_distance
                .into_iter()
                .map(DailyDistanceResponse::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeTotalResponse {
    pub mode: String,
    pub trip_count: u32,
    pub distance_meters: f64,
    pub duration_seconds: i64,
}

impl From<ModeTotal> for ModeTotalResponse {
    fn from(value: ModeTotal) -> Self {
        Self {
            mode: value.mode.as_str().to_owned(),
            trip_count: value.trip_count,
            distance_meters: value.distance_meters,
            duration_seconds: value.duration_seconds,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyDistanceResponse {
    pub date: String,
    pub distance_meters: f64,
    pub trip_count: u32,
    pub visit_count: u32,
}

impl From<DailyDistance> for DailyDistanceResponse {
    fn from(value: DailyDistance) -> Self {
        Self {
            date: value.date.to_iso(),
            distance_meters: value.distance_meters,
            trip_count: value.trip_count,
            visit_count: value.visit_count,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaySummaryResponse {
    pub date: String,
    pub distance_meters: f64,
    pub moving_seconds: i64,
    pub trip_count: u32,
    pub visit_count: u32,
    pub modes: Vec<String>,
    pub first_activity_at: Option<String>,
    pub last_activity_at: Option<String>,
    pub highlight_place: Option<String>,
}

impl From<DaySummary> for DaySummaryResponse {
    fn from(value: DaySummary) -> Self {
        Self {
            date: value.date.to_iso(),
            distance_meters: value.distance_meters,
            moving_seconds: value.moving_seconds,
            trip_count: value.trip_count,
            visit_count: value.visit_count,
            modes: value
                .modes
                .into_iter()
                .map(|mode| mode.as_str().to_owned())
                .collect(),
            first_activity_at: value.first_activity_at.map(|at| at.to_rfc3339()),
            last_activity_at: value.last_activity_at.map(|at| at.to_rfc3339()),
            highlight_place: value.highlight_place,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayDetailResponse {
    pub date: String,
    pub summary: DaySummaryResponse,
    pub entries: Vec<DayEntryResponse>,
    pub trace: Value,
}

impl From<DayDetail> for DayDetailResponse {
    fn from(value: DayDetail) -> Self {
        Self {
            date: value.date.to_iso(),
            summary: value.summary.into(),
            entries: value
                .entries
                .into_iter()
                .map(DayEntryResponse::from)
                .collect(),
            trace: trace_collection(&value.trace),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DayEntryResponse {
    #[serde(rename_all = "camelCase")]
    Stay {
        id: String,
        place_id: String,
        label: String,
        kind: String,
        coordinate: [f64; 2],
        started_at: String,
        ended_at: String,
        duration_seconds: i64,
        probability: f64,
    },
    #[serde(rename_all = "camelCase")]
    Journey {
        id: String,
        mode: String,
        started_at: String,
        ended_at: String,
        duration_seconds: i64,
        distance_meters: f64,
        start: [f64; 2],
        end: [f64; 2],
        probability: f64,
    },
}

impl From<DayEntry> for DayEntryResponse {
    fn from(value: DayEntry) -> Self {
        match value {
            DayEntry::Stay(entry) => Self::Stay {
                id: entry.id,
                place_id: entry.place_id,
                label: entry.label,
                kind: entry.kind.as_str().to_owned(),
                coordinate: position(entry.coordinate),
                started_at: entry.range.start().to_rfc3339(),
                ended_at: entry.range.end().to_rfc3339(),
                duration_seconds: entry.range.duration_seconds(),
                probability: entry.probability,
            },
            DayEntry::Journey(entry) => Self::Journey {
                id: entry.id,
                mode: entry.mode.as_str().to_owned(),
                started_at: entry.range.start().to_rfc3339(),
                ended_at: entry.range.end().to_rfc3339(),
                duration_seconds: entry.range.duration_seconds(),
                distance_meters: entry.distance_meters,
                start: position(entry.start),
                end: position(entry.end),
                probability: entry.probability,
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceResponse {
    pub place_id: String,
    pub label: String,
    pub kind: String,
    pub coordinate: [f64; 2],
    pub visit_count: u32,
    pub total_seconds: i64,
    pub first_seen: String,
    pub last_seen: String,
}

impl From<PlaceRanking> for PlaceResponse {
    fn from(value: PlaceRanking) -> Self {
        Self {
            place_id: value.place_id,
            label: value.label,
            kind: value.kind.as_str().to_owned(),
            coordinate: position(value.coordinate),
            visit_count: value.visit_count,
            total_seconds: value.total_seconds,
            first_seen: value.first_seen.to_rfc3339(),
            last_seen: value.last_seen.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsResponse {
    pub from: Option<String>,
    pub to: Option<String>,
    pub distance_meters: f64,
    pub moving_seconds: i64,
    pub tracked_days: u32,
    pub trip_count: u32,
    pub visit_count: u32,
    pub distinct_places: u32,
    pub modes: Vec<ModeTotalResponse>,
    pub busiest_day: Option<DailyDistanceResponse>,
}

impl From<RangeStats> for StatsResponse {
    fn from(value: RangeStats) -> Self {
        Self {
            from: value.from.map(|date| date.to_iso()),
            to: value.to.map(|date| date.to_iso()),
            distance_meters: value.distance_meters,
            moving_seconds: value.moving_seconds,
            tracked_days: value.tracked_days,
            trip_count: value.trip_count,
            visit_count: value.visit_count,
            distinct_places: value.distinct_places,
            modes: value
                .modes
                .into_iter()
                .map(ModeTotalResponse::from)
                .collect(),
            busiest_day: value.busiest_day.map(DailyDistanceResponse::from),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeocodeResponse {
    pub pending: u32,
    pub resolved: u32,
    pub failed: u32,
}

impl From<GeocodeReport> for GeocodeResponse {
    fn from(value: GeocodeReport) -> Self {
        Self {
            pending: value.pending,
            resolved: value.resolved,
            failed: value.failed,
        }
    }
}

pub fn trace_collection(lines: &[TraceLine]) -> Value {
    json!({
        "type": "FeatureCollection",
        "features": lines.iter().map(trace_feature).collect::<Vec<_>>(),
    })
}

fn trace_feature(line: &TraceLine) -> Value {
    let start = line.points.first().map(|point| point.at);
    let end = line.points.last().map(|point| point.at);

    json!({
        "type": "Feature",
        "id": line.id,
        "properties": {
            "id": line.id,
            "date": line.date.to_iso(),
            "mode": line.mode.as_ref().map(|mode| mode.as_str()),
            "startedAt": start.map(|at| at.to_rfc3339()),
            "endedAt": end.map(|at| at.to_rfc3339()),
            "timestamps": line
                .points
                .iter()
                .map(|point| point.at.utc().timestamp())
                .collect::<Vec<_>>(),
        },
        "geometry": {
            "type": "LineString",
            "coordinates": line
                .points
                .iter()
                .map(|point| position(point.coordinate))
                .collect::<Vec<_>>(),
        },
    })
}

fn position(coordinate: Coordinate) -> [f64; 2] {
    [coordinate.longitude(), coordinate.latitude()]
}
