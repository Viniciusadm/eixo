use async_trait::async_trait;

use crate::application::timeline::{DailyDistance, ModeTotal, PlaceRanking, RangeQuery};
use crate::domain::timeline::{Activity, PathSegment, Place, Visit};
use crate::domain::{Coordinate, LocalDate};
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TimelineCounters {
    pub first_date: Option<LocalDate>,
    pub last_date: Option<LocalDate>,
    pub tracked_days: u32,
    pub place_count: u32,
    pub visit_count: u32,
    pub activity_count: u32,
    pub trace_point_count: u32,
    pub total_distance_meters: f64,
}

#[async_trait]
pub trait TimelineRepository: Send + Sync {
    async fn counters(&self) -> Result<TimelineCounters, AppError>;
    async fn daily_distance(&self, query: &RangeQuery) -> Result<Vec<DailyDistance>, AppError>;
    async fn mode_totals(&self, query: &RangeQuery) -> Result<Vec<ModeTotal>, AppError>;
    async fn activities(&self, query: &RangeQuery) -> Result<Vec<Activity>, AppError>;
    async fn visits(&self, query: &RangeQuery) -> Result<Vec<(Visit, Place)>, AppError>;
    async fn path_segments(&self, query: &RangeQuery) -> Result<Vec<PathSegment>, AppError>;
    async fn place_rankings(&self, limit: u32) -> Result<Vec<PlaceRanking>, AppError>;
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ImportBatch {
    pub places: Vec<Place>,
    pub visits: Vec<Visit>,
    pub activities: Vec<Activity>,
    pub path_segments: Vec<PathSegment>,
    pub skipped: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingPlace {
    pub id: String,
    pub coordinate: Coordinate,
}

#[async_trait]
pub trait TimelineWriter: Send + Sync {
    async fn replace_all(&self, batch: ImportBatch, source: &str) -> Result<(), AppError>;
    async fn places_without_name(&self) -> Result<Vec<PendingPlace>, AppError>;
    async fn save_place_name(&self, place_id: &str, display_name: &str) -> Result<(), AppError>;
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RawTimeline {
    pub segments: Vec<RawSegment>,
    pub profile_places: Vec<RawProfilePlace>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawSegment {
    pub start_time: String,
    pub end_time: String,
    pub kind: RawSegmentKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RawSegmentKind {
    Path(Vec<RawTracePoint>),
    Visit(RawVisit),
    Activity(RawActivity),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawTracePoint {
    pub point: String,
    pub time: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawVisit {
    pub place_id: String,
    pub lat_lng: String,
    pub semantic_type: String,
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawActivity {
    pub start_lat_lng: String,
    pub end_lat_lng: String,
    pub distance_meters: f64,
    pub mode: String,
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawProfilePlace {
    pub lat_lng: String,
    pub label: Option<String>,
}

#[async_trait]
pub trait TimelineSource: Send + Sync {
    async fn load(&self) -> Result<RawTimeline, AppError>;
    fn describe(&self) -> String;
}

#[async_trait]
pub trait PlaceNameResolver: Send + Sync {
    async fn resolve(&self, coordinate: Coordinate) -> Result<Option<String>, AppError>;
}
