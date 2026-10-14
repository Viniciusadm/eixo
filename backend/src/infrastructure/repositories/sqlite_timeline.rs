use async_trait::async_trait;
use chrono::Utc;
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};
use uuid::Uuid;

use crate::application::timeline::{
    DailyDistance, ImportBatch, ModeTotal, PendingPlace, PlaceRanking, RangeQuery,
    TimelineCounters, TimelineRepository, TimelineWriter,
};
use crate::domain::timeline::{Activity, PathSegment, Place, TracePoint, Visit};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const INSERT_CHUNK: usize = 800;

#[derive(Clone)]
pub struct SqliteTimelineRepository {
    pool: SqlitePool,
}

impl SqliteTimelineRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn date_filter(query: &RangeQuery, alias: &str) -> (Vec<String>, Vec<String>) {
    let mut clauses = Vec::new();
    let mut binds = Vec::new();
    if let Some(from) = query.from {
        clauses.push(format!("{alias}local_date >= ?"));
        binds.push(from.to_iso());
    }
    if let Some(to) = query.to {
        clauses.push(format!("{alias}local_date <= ?"));
        binds.push(to.to_iso());
    }
    (clauses, binds)
}

fn mode_filter(query: &RangeQuery, alias: &str) -> (Vec<String>, Vec<String>) {
    if query.modes.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let placeholders = vec!["?"; query.modes.len()].join(", ");
    (
        vec![format!("{alias}mode IN ({placeholders})")],
        query
            .modes
            .iter()
            .map(|mode| mode.as_str().to_owned())
            .collect(),
    )
}

fn where_clause(clauses: &[String]) -> String {
    if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    }
}

macro_rules! bind_all {
    ($query:expr, $binds:expr) => {{
        let mut query = $query;
        for bind in $binds {
            query = query.bind(bind);
        }
        query
    }};
}

#[async_trait]
impl TimelineRepository for SqliteTimelineRepository {
    async fn counters(&self) -> Result<TimelineCounters, AppError> {
        let row = sqlx::query(
            r#"
            WITH tracked AS (
                SELECT local_date FROM visits
                UNION SELECT local_date FROM activities
                UNION SELECT local_date FROM path_segments
            )
            SELECT
                (SELECT MIN(local_date) FROM tracked) AS first_date,
                (SELECT MAX(local_date) FROM tracked) AS last_date,
                (SELECT COUNT(*) FROM tracked) AS tracked_days,
                (SELECT COUNT(*) FROM places) AS place_count,
                (SELECT COUNT(*) FROM visits) AS visit_count,
                (SELECT COUNT(*) FROM activities) AS activity_count,
                (SELECT COUNT(*) FROM path_points) AS trace_point_count,
                (SELECT COALESCE(SUM(distance_meters), 0.0) FROM activities) AS total_distance
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let first_date: Option<String> = row.try_get("first_date")?;
        let last_date: Option<String> = row.try_get("last_date")?;

        Ok(TimelineCounters {
            first_date: first_date.as_deref().map(mapper::local_date).transpose()?,
            last_date: last_date.as_deref().map(mapper::local_date).transpose()?,
            tracked_days: row.try_get::<i64, _>("tracked_days")? as u32,
            place_count: row.try_get::<i64, _>("place_count")? as u32,
            visit_count: row.try_get::<i64, _>("visit_count")? as u32,
            activity_count: row.try_get::<i64, _>("activity_count")? as u32,
            trace_point_count: row.try_get::<i64, _>("trace_point_count")? as u32,
            total_distance_meters: row.try_get("total_distance")?,
        })
    }

    async fn daily_distance(&self, query: &RangeQuery) -> Result<Vec<DailyDistance>, AppError> {
        let (mut clauses, mut binds) = date_filter(query, "");
        let (mode_clauses, mode_binds) = mode_filter(query, "");
        clauses.extend(mode_clauses);
        let activity_where = where_clause(&clauses);
        binds.extend(mode_binds);

        let (visit_clauses, visit_binds) = date_filter(query, "");
        let visit_where = where_clause(&visit_clauses);

        let sql = format!(
            r#"
            SELECT local_date, SUM(distance_meters) AS distance, COUNT(*) AS trips, 0 AS visits
            FROM activities{activity_where}
            GROUP BY local_date
            UNION ALL
            SELECT local_date, 0.0 AS distance, 0 AS trips, COUNT(*) AS visits
            FROM visits{visit_where}
            GROUP BY local_date
            "#
        );

        binds.extend(visit_binds);
        let rows = bind_all!(sqlx::query(&sql), binds)
            .fetch_all(&self.pool)
            .await?;

        let mut days: Vec<DailyDistance> = Vec::new();
        for row in rows {
            let date = mapper::local_date(row.try_get("local_date")?)?;
            let distance: f64 = row.try_get("distance")?;
            let trips = row.try_get::<i64, _>("trips")? as u32;
            let visits = row.try_get::<i64, _>("visits")? as u32;

            match days.iter_mut().find(|day| day.date == date) {
                Some(day) => {
                    day.distance_meters += distance;
                    day.trip_count += trips;
                    day.visit_count += visits;
                }
                None => days.push(DailyDistance {
                    date,
                    distance_meters: distance,
                    trip_count: trips,
                    visit_count: visits,
                }),
            }
        }
        days.sort_by_key(|day| day.date);
        Ok(days)
    }

    async fn mode_totals(&self, query: &RangeQuery) -> Result<Vec<ModeTotal>, AppError> {
        let (mut clauses, mut binds) = date_filter(query, "");
        let (mode_clauses, mode_binds) = mode_filter(query, "");
        clauses.extend(mode_clauses);
        binds.extend(mode_binds);

        let sql = format!(
            r#"
            SELECT mode, COUNT(*) AS trips,
                   SUM(distance_meters) AS distance,
                   SUM(duration_seconds) AS duration
            FROM activities{}
            GROUP BY mode
            ORDER BY distance DESC
            "#,
            where_clause(&clauses)
        );

        let rows = bind_all!(sqlx::query(&sql), binds)
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter()
            .map(|row| {
                Ok(ModeTotal {
                    mode: mapper::travel_mode(row.try_get("mode")?),
                    trip_count: row.try_get::<i64, _>("trips")? as u32,
                    distance_meters: row.try_get("distance")?,
                    duration_seconds: row.try_get("duration")?,
                })
            })
            .collect()
    }

    async fn activities(&self, query: &RangeQuery) -> Result<Vec<Activity>, AppError> {
        let (mut clauses, mut binds) = date_filter(query, "");
        let (mode_clauses, mode_binds) = mode_filter(query, "");
        clauses.extend(mode_clauses);
        binds.extend(mode_binds);

        let sql = format!(
            r#"
            SELECT id, started_at, ended_at, offset_minutes, local_date,
                   start_lat, start_lng, end_lat, end_lng,
                   distance_meters, mode, probability
            FROM activities{}
            ORDER BY started_at
            "#,
            where_clause(&clauses)
        );

        let rows = bind_all!(sqlx::query(&sql), binds)
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter()
            .map(|row| {
                let offset: i64 = row.try_get("offset_minutes")?;
                Ok(Activity {
                    id: mapper::id(row.try_get("id")?)?,
                    range: mapper::time_range(
                        row.try_get("started_at")?,
                        row.try_get("ended_at")?,
                        offset,
                    )?,
                    local_date: mapper::local_date(row.try_get("local_date")?)?,
                    start: mapper::coordinate(
                        row.try_get("start_lat")?,
                        row.try_get("start_lng")?,
                    )?,
                    end: mapper::coordinate(row.try_get("end_lat")?, row.try_get("end_lng")?)?,
                    distance_meters: row.try_get("distance_meters")?,
                    mode: mapper::travel_mode(row.try_get("mode")?),
                    probability: row.try_get("probability")?,
                })
            })
            .collect()
    }

    async fn visits(&self, query: &RangeQuery) -> Result<Vec<(Visit, Place)>, AppError> {
        let (clauses, binds) = date_filter(query, "v.");

        let sql = format!(
            r#"
            SELECT v.id, v.place_id, v.started_at, v.ended_at, v.offset_minutes,
                   v.local_date, v.probability,
                   p.google_place_id, p.lat, p.lng, p.kind, p.display_name
            FROM visits v
            JOIN places p ON p.id = v.place_id{}
            ORDER BY v.started_at
            "#,
            where_clause(&clauses)
        );

        let rows = bind_all!(sqlx::query(&sql), binds)
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter()
            .map(|row| {
                let offset: i64 = row.try_get("offset_minutes")?;
                let place = Place {
                    id: mapper::id(row.try_get("place_id")?)?,
                    google_place_id: row.try_get("google_place_id")?,
                    coordinate: mapper::coordinate(row.try_get("lat")?, row.try_get("lng")?)?,
                    kind: mapper::place_kind(row.try_get("kind")?),
                    display_name: row.try_get("display_name")?,
                };
                let visit = Visit {
                    id: mapper::id(row.try_get("id")?)?,
                    place_id: place.id.clone(),
                    range: mapper::time_range(
                        row.try_get("started_at")?,
                        row.try_get("ended_at")?,
                        offset,
                    )?,
                    local_date: mapper::local_date(row.try_get("local_date")?)?,
                    probability: row.try_get("probability")?,
                };
                Ok((visit, place))
            })
            .collect()
    }

    async fn path_segments(&self, query: &RangeQuery) -> Result<Vec<PathSegment>, AppError> {
        let (clauses, binds) = date_filter(query, "s.");

        let sql = format!(
            r#"
            SELECT s.id AS segment_id, s.started_at, s.ended_at,
                   s.offset_minutes AS segment_offset, s.local_date,
                   p.recorded_at, p.offset_minutes AS point_offset, p.lat, p.lng
            FROM path_segments s
            JOIN path_points p ON p.segment_id = s.id{}
            ORDER BY s.started_at, p.recorded_at
            "#,
            where_clause(&clauses)
        );

        let rows = bind_all!(sqlx::query(&sql), binds)
            .fetch_all(&self.pool)
            .await?;

        let mut segments: Vec<PathSegment> = Vec::new();
        for row in rows {
            let segment_id: String = row.try_get("segment_id")?;
            let point = TracePoint {
                at: mapper::instant(row.try_get("recorded_at")?, row.try_get("point_offset")?)?,
                coordinate: mapper::coordinate(row.try_get("lat")?, row.try_get("lng")?)?,
            };

            match segments.last_mut() {
                Some(segment) if segment.id.as_str() == segment_id => segment.points.push(point),
                _ => segments.push(PathSegment {
                    id: mapper::id(segment_id)?,
                    range: mapper::time_range(
                        row.try_get("started_at")?,
                        row.try_get("ended_at")?,
                        row.try_get("segment_offset")?,
                    )?,
                    local_date: mapper::local_date(row.try_get("local_date")?)?,
                    points: vec![point],
                }),
            }
        }
        Ok(segments)
    }

    async fn place_rankings(&self, limit: u32) -> Result<Vec<PlaceRanking>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT p.google_place_id, p.lat, p.lng, p.kind, p.display_name,
                   COUNT(v.id) AS visit_count,
                   SUM(v.duration_seconds) AS total_seconds,
                   MIN(v.started_at) AS first_seen,
                   MAX(v.ended_at) AS last_seen,
                   MAX(v.offset_minutes) AS offset_minutes
            FROM places p
            JOIN visits v ON v.place_id = p.id
            GROUP BY p.id
            ORDER BY total_seconds DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let offset: i64 = row.try_get("offset_minutes")?;
                let coordinate = mapper::coordinate(row.try_get("lat")?, row.try_get("lng")?)?;
                let display_name: Option<String> = row.try_get("display_name")?;

                Ok(PlaceRanking {
                    place_id: row.try_get("google_place_id")?,
                    label: display_name.unwrap_or_else(|| {
                        format!(
                            "{:.5}, {:.5}",
                            coordinate.latitude(),
                            coordinate.longitude()
                        )
                    }),
                    kind: mapper::place_kind(row.try_get("kind")?),
                    coordinate,
                    visit_count: row.try_get::<i64, _>("visit_count")? as u32,
                    total_seconds: row.try_get("total_seconds")?,
                    first_seen: mapper::instant(row.try_get("first_seen")?, offset)?,
                    last_seen: mapper::instant(row.try_get("last_seen")?, offset)?,
                })
            })
            .collect()
    }
}

#[async_trait]
impl TimelineWriter for SqliteTimelineRepository {
    async fn replace_all(&self, batch: ImportBatch, source: &str) -> Result<(), AppError> {
        let mut transaction = self.pool.begin().await?;

        for table in [
            "path_points",
            "path_segments",
            "activities",
            "visits",
            "places",
        ] {
            sqlx::query(&format!("DELETE FROM {table}"))
                .execute(&mut *transaction)
                .await?;
        }

        for chunk in batch.places.chunks(INSERT_CHUNK) {
            let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
                "INSERT INTO places (id, google_place_id, lat, lng, kind, display_name) ",
            );
            builder.push_values(chunk, |mut row, place| {
                row.push_bind(place.id.as_str().to_owned())
                    .push_bind(place.google_place_id.clone())
                    .push_bind(place.coordinate.latitude())
                    .push_bind(place.coordinate.longitude())
                    .push_bind(place.kind.as_str().to_owned())
                    .push_bind(place.display_name.clone());
            });
            builder.build().execute(&mut *transaction).await?;
        }

        for chunk in batch.visits.chunks(INSERT_CHUNK) {
            let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
                "INSERT INTO visits (id, place_id, started_at, ended_at, offset_minutes, local_date, duration_seconds, probability) ",
            );
            builder.push_values(chunk, |mut row, visit| {
                row.push_bind(visit.id.as_str().to_owned())
                    .push_bind(visit.place_id.as_str().to_owned())
                    .push_bind(visit.range.start().to_storage())
                    .push_bind(visit.range.end().to_storage())
                    .push_bind(visit.range.start().offset_minutes() as i64)
                    .push_bind(visit.local_date.to_iso())
                    .push_bind(visit.duration_seconds())
                    .push_bind(visit.probability);
            });
            builder.build().execute(&mut *transaction).await?;
        }

        for chunk in batch.activities.chunks(INSERT_CHUNK) {
            let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
                "INSERT INTO activities (id, started_at, ended_at, offset_minutes, local_date, start_lat, start_lng, end_lat, end_lng, duration_seconds, distance_meters, mode, probability) ",
            );
            builder.push_values(chunk, |mut row, activity| {
                row.push_bind(activity.id.as_str().to_owned())
                    .push_bind(activity.range.start().to_storage())
                    .push_bind(activity.range.end().to_storage())
                    .push_bind(activity.range.start().offset_minutes() as i64)
                    .push_bind(activity.local_date.to_iso())
                    .push_bind(activity.start.latitude())
                    .push_bind(activity.start.longitude())
                    .push_bind(activity.end.latitude())
                    .push_bind(activity.end.longitude())
                    .push_bind(activity.range.duration_seconds())
                    .push_bind(activity.distance_meters)
                    .push_bind(activity.mode.as_str().to_owned())
                    .push_bind(activity.probability);
            });
            builder.build().execute(&mut *transaction).await?;
        }

        for chunk in batch.path_segments.chunks(INSERT_CHUNK) {
            let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
                "INSERT INTO path_segments (id, started_at, ended_at, offset_minutes, local_date) ",
            );
            builder.push_values(chunk, |mut row, segment| {
                row.push_bind(segment.id.as_str().to_owned())
                    .push_bind(segment.range.start().to_storage())
                    .push_bind(segment.range.end().to_storage())
                    .push_bind(segment.range.start().offset_minutes() as i64)
                    .push_bind(segment.local_date.to_iso());
            });
            builder.build().execute(&mut *transaction).await?;
        }

        let points: Vec<(String, &TracePoint)> = batch
            .path_segments
            .iter()
            .flat_map(|segment| {
                segment
                    .points
                    .iter()
                    .map(|point| (segment.id.as_str().to_owned(), point))
            })
            .collect();

        for chunk in points.chunks(INSERT_CHUNK) {
            let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
                "INSERT INTO path_points (segment_id, recorded_at, offset_minutes, local_date, lat, lng) ",
            );
            builder.push_values(chunk, |mut row, (segment_id, point)| {
                row.push_bind(segment_id.clone())
                    .push_bind(point.at.to_storage())
                    .push_bind(point.at.offset_minutes() as i64)
                    .push_bind(point.at.local_date().to_iso())
                    .push_bind(point.coordinate.latitude())
                    .push_bind(point.coordinate.longitude());
            });
            builder.build().execute(&mut *transaction).await?;
        }

        sqlx::query(
            "INSERT INTO import_runs (id, source, imported_at, places, visits, activities, path_segments, trace_points, skipped) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(source)
        .bind(Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
        .bind(batch.places.len() as i64)
        .bind(batch.visits.len() as i64)
        .bind(batch.activities.len() as i64)
        .bind(batch.path_segments.len() as i64)
        .bind(points.len() as i64)
        .bind(batch.skipped as i64)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(())
    }

    async fn places_without_name(&self) -> Result<Vec<PendingPlace>, AppError> {
        let rows = sqlx::query(
            "SELECT id, lat, lng FROM places WHERE display_name IS NULL ORDER BY google_place_id",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(PendingPlace {
                    id: row.try_get("id")?,
                    coordinate: mapper::coordinate(row.try_get("lat")?, row.try_get("lng")?)?,
                })
            })
            .collect()
    }

    async fn save_place_name(&self, place_id: &str, display_name: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE places SET display_name = ?, resolved_at = ? WHERE id = ?")
            .bind(display_name)
            .bind(Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
            .bind(place_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
