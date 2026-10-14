use std::collections::HashMap;

use uuid::Uuid;

use crate::application::timeline::{
    DailyDistance, DayDetail, DayEntry, DaySummary, GeocodeReport, ImportBatch, ImportReport,
    JourneyEntry, PendingPlace, PlaceNameResolver, PlaceRanking, RangeQuery, RangeStats,
    RawActivity, RawProfilePlace, RawSegment, RawSegmentKind, RawTimeline, RawTracePoint, RawVisit,
    StayEntry, TimelineOverview, TimelineRepository, TimelineSource, TimelineWriter, TraceLine,
    TracePointView,
};
use crate::domain::timeline::{Activity, PathSegment, Place, TracePoint, Visit};
use crate::domain::{Coordinate, DomainId, Instant, LocalDate, PlaceKind, TimeRange, TravelMode};
use crate::errors::AppError;

const PROFILE_MATCH_METERS: f64 = 120.0;

pub struct TimelineQueries<R> {
    timeline: R,
}

impl<R> TimelineQueries<R>
where
    R: TimelineRepository,
{
    pub fn new(timeline: R) -> Self {
        Self { timeline }
    }

    pub async fn overview(&self) -> Result<TimelineOverview, AppError> {
        let unbounded = RangeQuery::unbounded();
        let counters = self.timeline.counters().await?;

        Ok(TimelineOverview {
            first_date: counters.first_date,
            last_date: counters.last_date,
            tracked_days: counters.tracked_days,
            total_distance_meters: counters.total_distance_meters,
            visit_count: counters.visit_count,
            place_count: counters.place_count,
            trace_point_count: counters.trace_point_count,
            modes: self.timeline.mode_totals(&unbounded).await?,
            daily_distance: self.timeline.daily_distance(&unbounded).await?,
        })
    }

    pub async fn days(&self, query: RangeQuery) -> Result<Vec<DaySummary>, AppError> {
        let activities = self.timeline.activities(&query).await?;
        let visits = self.timeline.visits(&query).await?;
        Ok(summarise_days(&activities, &visits))
    }

    pub async fn day(&self, date: LocalDate) -> Result<DayDetail, AppError> {
        let query = RangeQuery {
            from: Some(date),
            to: Some(date),
            modes: Vec::new(),
        };
        let activities = self.timeline.activities(&query).await?;
        let visits = self.timeline.visits(&query).await?;
        let segments = self.timeline.path_segments(&query).await?;

        if activities.is_empty() && visits.is_empty() && segments.is_empty() {
            return Err(AppError::NotFound(
                "no timeline records for this day".to_owned(),
            ));
        }

        let summary = summarise_days(&activities, &visits)
            .into_iter()
            .next()
            .unwrap_or_else(|| empty_summary(date));

        Ok(DayDetail {
            date,
            summary,
            entries: build_entries(&activities, &visits),
            trace: build_trace(&segments, &activities, &[]),
        })
    }

    pub async fn trace(&self, query: RangeQuery) -> Result<Vec<TraceLine>, AppError> {
        let segments = self.timeline.path_segments(&query).await?;
        let activities = self
            .timeline
            .activities(&RangeQuery {
                modes: Vec::new(),
                ..query.clone()
            })
            .await?;
        Ok(build_trace(&segments, &activities, &query.modes))
    }

    pub async fn places(&self, limit: u32) -> Result<Vec<PlaceRanking>, AppError> {
        self.timeline.place_rankings(limit.clamp(1, 500)).await
    }

    pub async fn stats(&self, query: RangeQuery) -> Result<RangeStats, AppError> {
        let activities = self.timeline.activities(&query).await?;
        let visits = self.timeline.visits(&query).await?;
        let daily = self.timeline.daily_distance(&query).await?;

        let mut distinct_places: Vec<&str> = visits
            .iter()
            .map(|(_, place)| place.google_place_id.as_str())
            .collect();
        distinct_places.sort_unstable();
        distinct_places.dedup();

        let mut tracked: Vec<LocalDate> = activities
            .iter()
            .map(|activity| activity.local_date)
            .chain(visits.iter().map(|(visit, _)| visit.local_date))
            .collect();
        tracked.sort_unstable();
        tracked.dedup();

        Ok(RangeStats {
            from: query.from,
            to: query.to,
            distance_meters: activities
                .iter()
                .map(|activity| activity.distance_meters)
                .sum(),
            moving_seconds: activities
                .iter()
                .map(|activity| activity.range.duration_seconds())
                .sum(),
            tracked_days: tracked.len() as u32,
            trip_count: activities.len() as u32,
            visit_count: visits.len() as u32,
            distinct_places: distinct_places.len() as u32,
            modes: self.timeline.mode_totals(&query).await?,
            busiest_day: daily
                .into_iter()
                .max_by(|left, right| left.distance_meters.total_cmp(&right.distance_meters)),
        })
    }

    pub async fn daily_distance(&self, query: RangeQuery) -> Result<Vec<DailyDistance>, AppError> {
        self.timeline.daily_distance(&query).await
    }
}

fn empty_summary(date: LocalDate) -> DaySummary {
    DaySummary {
        date,
        distance_meters: 0.0,
        moving_seconds: 0,
        trip_count: 0,
        visit_count: 0,
        modes: Vec::new(),
        first_activity_at: None,
        last_activity_at: None,
        highlight_place: None,
    }
}

pub fn summarise_days(activities: &[Activity], visits: &[(Visit, Place)]) -> Vec<DaySummary> {
    let mut days: HashMap<LocalDate, DaySummary> = HashMap::new();
    let mut longest_stay: HashMap<LocalDate, (i64, String)> = HashMap::new();

    for activity in activities {
        let summary = days
            .entry(activity.local_date)
            .or_insert_with(|| empty_summary(activity.local_date));
        summary.distance_meters += activity.distance_meters;
        summary.moving_seconds += activity.range.duration_seconds();
        summary.trip_count += 1;
        if !summary.modes.contains(&activity.mode) {
            summary.modes.push(activity.mode.clone());
        }
        widen(summary, activity.range.start(), activity.range.end());
    }

    for (visit, place) in visits {
        let summary = days
            .entry(visit.local_date)
            .or_insert_with(|| empty_summary(visit.local_date));
        summary.visit_count += 1;
        widen(summary, visit.range.start(), visit.range.end());

        let duration = visit.duration_seconds();
        let entry = longest_stay
            .entry(visit.local_date)
            .or_insert_with(|| (0, String::new()));
        if duration > entry.0 && place.kind != PlaceKind::Home {
            *entry = (duration, place.label());
        }
    }

    let mut summaries: Vec<DaySummary> = days
        .into_values()
        .map(|mut summary| {
            summary.highlight_place = longest_stay
                .get(&summary.date)
                .filter(|(duration, _)| *duration > 0)
                .map(|(_, label)| label.clone());
            summary.modes.sort_by_key(|mode| mode.as_str().to_owned());
            summary
        })
        .collect();
    summaries.sort_by_key(|summary| summary.date);
    summaries
}

fn widen(summary: &mut DaySummary, start: Instant, end: Instant) {
    summary.first_activity_at = Some(match summary.first_activity_at {
        Some(current) if current <= start => current,
        _ => start,
    });
    summary.last_activity_at = Some(match summary.last_activity_at {
        Some(current) if current >= end => current,
        _ => end,
    });
}

pub fn build_entries(activities: &[Activity], visits: &[(Visit, Place)]) -> Vec<DayEntry> {
    let mut entries: Vec<DayEntry> = visits
        .iter()
        .map(|(visit, place)| {
            DayEntry::Stay(StayEntry {
                id: visit.id.as_str().to_owned(),
                place_id: place.google_place_id.clone(),
                label: place.label(),
                kind: place.kind,
                coordinate: place.coordinate,
                range: visit.range,
                probability: visit.probability,
            })
        })
        .chain(activities.iter().map(|activity| {
            DayEntry::Journey(JourneyEntry {
                id: activity.id.as_str().to_owned(),
                mode: activity.mode.clone(),
                range: activity.range,
                start: activity.start,
                end: activity.end,
                distance_meters: activity.distance_meters,
                probability: activity.probability,
            })
        }))
        .collect();

    entries.sort_by_key(|entry| entry.started_at());
    entries
}

pub fn build_trace(
    segments: &[PathSegment],
    activities: &[Activity],
    modes: &[TravelMode],
) -> Vec<TraceLine> {
    let mut lines = Vec::new();

    for segment in segments {
        let mut current: Vec<TracePointView> = Vec::new();
        let mut current_mode: Option<TravelMode> = None;

        for point in &segment.points {
            let mode = mode_at(activities, point.at);
            if current.is_empty() || mode == current_mode {
                current.push(TracePointView {
                    at: point.at,
                    coordinate: point.coordinate,
                });
                current_mode = mode;
                continue;
            }

            let boundary = *current.last().expect("run is not empty");
            push_line(&mut lines, segment, current_mode.clone(), current, modes);
            current = vec![
                boundary,
                TracePointView {
                    at: point.at,
                    coordinate: point.coordinate,
                },
            ];
            current_mode = mode;
        }

        push_line(&mut lines, segment, current_mode, current, modes);
    }

    lines
}

fn push_line(
    lines: &mut Vec<TraceLine>,
    segment: &PathSegment,
    mode: Option<TravelMode>,
    points: Vec<TracePointView>,
    modes: &[TravelMode],
) {
    if points.len() < 2 {
        return;
    }
    if !modes.is_empty() && !mode.as_ref().is_some_and(|mode| modes.contains(mode)) {
        return;
    }

    lines.push(TraceLine {
        id: format!("{}-{}", segment.id.as_str(), lines.len()),
        date: segment.local_date,
        mode,
        points,
    });
}

fn mode_at(activities: &[Activity], at: Instant) -> Option<TravelMode> {
    activities
        .iter()
        .find(|activity| {
            activity.range.start().utc() <= at.utc() && at.utc() <= activity.range.end().utc()
        })
        .map(|activity| activity.mode.clone())
}

pub struct ImportTimeline<S, W> {
    source: S,
    writer: W,
}

impl<S, W> ImportTimeline<S, W>
where
    S: TimelineSource,
    W: TimelineWriter,
{
    pub fn new(source: S, writer: W) -> Self {
        Self { source, writer }
    }

    pub async fn execute(&self) -> Result<ImportReport, AppError> {
        let raw = self.source.load().await?;
        let batch = build_batch(raw);
        let report = ImportReport {
            segments: (batch.visits.len() + batch.activities.len() + batch.path_segments.len())
                as u32,
            places: batch.places.len() as u32,
            visits: batch.visits.len() as u32,
            activities: batch.activities.len() as u32,
            path_segments: batch.path_segments.len() as u32,
            trace_points: batch
                .path_segments
                .iter()
                .map(|segment| segment.points.len() as u32)
                .sum(),
            skipped: batch.skipped,
        };
        self.writer
            .replace_all(batch, &self.source.describe())
            .await?;
        Ok(report)
    }
}

pub struct ResolvePlaceNames<W, G> {
    writer: W,
    resolver: G,
}

impl<W, G> ResolvePlaceNames<W, G>
where
    W: TimelineWriter,
    G: PlaceNameResolver,
{
    pub fn new(writer: W, resolver: G) -> Self {
        Self { writer, resolver }
    }

    pub async fn execute(&self) -> Result<GeocodeReport, AppError> {
        let pending = self.writer.places_without_name().await?;
        let mut report = GeocodeReport {
            pending: pending.len() as u32,
            ..GeocodeReport::default()
        };

        for PendingPlace { id, coordinate } in pending {
            match self.resolver.resolve(coordinate).await {
                Ok(Some(display_name)) => {
                    self.writer.save_place_name(&id, &display_name).await?;
                    report.resolved += 1;
                }
                Ok(None) => report.failed += 1,
                Err(error) => {
                    tracing::warn!("failed to resolve place {id}: {error}");
                    report.failed += 1;
                }
            }
        }

        Ok(report)
    }
}

pub fn parse_coordinate(value: &str) -> Option<Coordinate> {
    let cleaned = value.replace('\u{00b0}', "");
    let (latitude, longitude) = cleaned.split_once(',')?;
    Coordinate::new(
        latitude.trim().parse().ok()?,
        longitude.trim().parse().ok()?,
    )
    .ok()
}

pub fn build_batch(raw: RawTimeline) -> ImportBatch {
    let mut builder = BatchBuilder::default();

    for segment in raw.segments {
        builder.absorb(segment);
    }

    builder.finish(&raw.profile_places)
}

#[derive(Default)]
struct BatchBuilder {
    places: HashMap<String, PlaceDraft>,
    visits: Vec<Visit>,
    activities: Vec<Activity>,
    path_segments: Vec<PathSegment>,
    skipped: u32,
}

struct PlaceDraft {
    id: DomainId,
    google_place_id: String,
    coordinate: Coordinate,
    kind_votes: HashMap<PlaceKind, u32>,
}

impl BatchBuilder {
    fn absorb(&mut self, segment: RawSegment) {
        let range = match (
            Instant::from_rfc3339(&segment.start_time),
            Instant::from_rfc3339(&segment.end_time),
        ) {
            (Ok(start), Ok(end)) => TimeRange::new(start, end).ok(),
            _ => None,
        };
        let Some(range) = range else {
            self.skipped += 1;
            return;
        };

        match segment.kind {
            RawSegmentKind::Visit(visit) => self.absorb_visit(range, visit),
            RawSegmentKind::Activity(activity) => self.absorb_activity(range, activity),
            RawSegmentKind::Path(points) => self.absorb_path(range, points),
        }
    }

    fn absorb_visit(&mut self, range: TimeRange, visit: RawVisit) {
        let Some(coordinate) = parse_coordinate(&visit.lat_lng) else {
            self.skipped += 1;
            return;
        };
        if visit.place_id.trim().is_empty() {
            self.skipped += 1;
            return;
        }

        let kind = PlaceKind::from_google(&visit.semantic_type);
        let draft = self
            .places
            .entry(visit.place_id.clone())
            .or_insert_with(|| PlaceDraft {
                id: new_id(),
                google_place_id: visit.place_id.clone(),
                coordinate,
                kind_votes: HashMap::new(),
            });
        *draft.kind_votes.entry(kind).or_insert(0) += 1;

        self.visits.push(Visit {
            id: new_id(),
            place_id: draft.id.clone(),
            local_date: range.start().local_date(),
            range,
            probability: visit.probability,
        });
    }

    fn absorb_activity(&mut self, range: TimeRange, activity: RawActivity) {
        let (Some(start), Some(end)) = (
            parse_coordinate(&activity.start_lat_lng),
            parse_coordinate(&activity.end_lat_lng),
        ) else {
            self.skipped += 1;
            return;
        };

        self.activities.push(Activity {
            id: new_id(),
            local_date: range.start().local_date(),
            range,
            start,
            end,
            distance_meters: if activity.distance_meters.is_finite()
                && activity.distance_meters >= 0.0
            {
                activity.distance_meters
            } else {
                start.distance_meters(&end)
            },
            mode: TravelMode::from_google(&activity.mode),
            probability: activity.probability,
        });
    }

    fn absorb_path(&mut self, range: TimeRange, points: Vec<RawTracePoint>) {
        let mut traced = Vec::with_capacity(points.len());
        for point in points {
            match (
                parse_coordinate(&point.point),
                Instant::from_rfc3339(&point.time),
            ) {
                (Some(coordinate), Ok(at)) => traced.push(TracePoint { at, coordinate }),
                _ => self.skipped += 1,
            }
        }
        if traced.is_empty() {
            return;
        }
        traced.sort_by_key(|point| point.at);

        self.path_segments.push(PathSegment {
            id: new_id(),
            local_date: traced[0].at.local_date(),
            range,
            points: traced,
        });
    }

    fn finish(self, profile_places: &[RawProfilePlace]) -> ImportBatch {
        let labelled: Vec<(Coordinate, PlaceKind)> = profile_places
            .iter()
            .filter_map(|place| {
                let coordinate = parse_coordinate(&place.lat_lng)?;
                let kind = PlaceKind::from_google(place.label.as_deref().unwrap_or_default());
                (kind != PlaceKind::Unknown).then_some((coordinate, kind))
            })
            .collect();

        let places = self
            .places
            .into_values()
            .map(|draft| {
                let voted = draft
                    .kind_votes
                    .iter()
                    .filter(|(kind, _)| **kind != PlaceKind::Unknown)
                    .max_by_key(|(_, votes)| **votes)
                    .map(|(kind, _)| *kind);
                let kind = voted
                    .or_else(|| nearest_label(&labelled, draft.coordinate))
                    .unwrap_or(PlaceKind::Unknown);

                Place {
                    id: draft.id,
                    google_place_id: draft.google_place_id,
                    coordinate: draft.coordinate,
                    kind,
                    display_name: None,
                }
            })
            .collect();

        ImportBatch {
            places,
            visits: self.visits,
            activities: self.activities,
            path_segments: self.path_segments,
            skipped: self.skipped,
        }
    }
}

fn nearest_label(
    labelled: &[(Coordinate, PlaceKind)],
    coordinate: Coordinate,
) -> Option<PlaceKind> {
    labelled
        .iter()
        .map(|(candidate, kind)| (candidate.distance_meters(&coordinate), *kind))
        .filter(|(distance, _)| *distance <= PROFILE_MATCH_METERS)
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, kind)| kind)
}

fn new_id() -> DomainId {
    DomainId::new(Uuid::new_v4().to_string()).expect("uuid is never empty")
}
