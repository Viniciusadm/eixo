use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use timeline_api::application::timeline::{
    build_batch, build_trace, parse_coordinate, summarise_days, ImportBatch, ImportTimeline,
    PendingPlace, PlaceNameResolver, RawActivity, RawProfilePlace, RawSegment, RawSegmentKind,
    RawTimeline, RawTracePoint, RawVisit, ResolvePlaceNames, TimelineSource, TimelineWriter,
};
use timeline_api::domain::timeline::{Activity, PathSegment, Place, TracePoint, Visit};
use timeline_api::domain::{
    Coordinate, DomainId, Instant, LocalDate, PlaceKind, TimeRange, TravelMode,
};
use timeline_api::errors::AppError;

#[test]
fn parses_the_degree_marked_coordinates_the_export_uses() {
    let coordinate = parse_coordinate("48.8583701°, 2.2944813°").unwrap();

    assert!((coordinate.latitude() - 48.8583701).abs() < 1e-9);
    assert!((coordinate.longitude() - 2.2944813).abs() < 1e-9);
}

#[test]
fn rejects_coordinates_that_are_not_usable() {
    assert!(parse_coordinate("").is_none());
    assert!(parse_coordinate("48.85").is_none());
    assert!(parse_coordinate("north, west").is_none());
    assert!(parse_coordinate("-115.0°, 2.0°").is_none());
}

#[test]
fn maps_every_travel_mode_present_in_the_export() {
    assert_eq!(TravelMode::from_google("WALKING"), TravelMode::Walking);
    assert_eq!(
        TravelMode::from_google("IN_PASSENGER_VEHICLE"),
        TravelMode::PassengerVehicle
    );
    assert_eq!(TravelMode::from_google("IN_SUBWAY"), TravelMode::Subway);
    assert_eq!(TravelMode::from_google("IN_BUS"), TravelMode::Bus);
    assert_eq!(TravelMode::from_google("CYCLING"), TravelMode::Cycling);
    assert_eq!(TravelMode::from_google("IN_TRAIN"), TravelMode::Train);
    assert_eq!(TravelMode::from_google("IN_TAXI"), TravelMode::Taxi);
    assert_eq!(
        TravelMode::from_google("IN_HOT_AIR_BALLOON"),
        TravelMode::Other("IN_HOT_AIR_BALLOON".to_owned())
    );
}

#[test]
fn a_day_follows_the_offset_recorded_in_the_timestamp() {
    let instant = Instant::from_rfc3339("2024-12-02T01:30:00.000-03:00").unwrap();
    assert_eq!(instant.local_date().to_iso(), "2024-12-02");

    let midnight_crossing = Instant::from_rfc3339("2024-12-02T01:30:00.000Z").unwrap();
    assert_eq!(midnight_crossing.local_date().to_iso(), "2024-12-02");

    let before_midnight = Instant::from_utc(
        Instant::from_rfc3339("2024-12-02T01:30:00.000Z")
            .unwrap()
            .utc(),
        -180,
    );
    assert_eq!(before_midnight.local_date().to_iso(), "2024-12-01");
}

#[test]
fn haversine_matches_a_known_distance() {
    let paris = Coordinate::new(48.8566, 2.3522).unwrap();
    let london = Coordinate::new(51.5074, -0.1278).unwrap();

    let kilometres = paris.distance_meters(&london) / 1000.0;

    assert!((kilometres - 343.5).abs() < 1.0, "got {kilometres} km");
}

#[test]
fn import_deduplicates_places_and_keeps_the_inferred_label() {
    let batch = build_batch(RawTimeline {
        segments: vec![
            visit_segment("place-a", "INFERRED_HOME", "08:00", "10:00"),
            visit_segment("place-a", "UNKNOWN", "12:00", "13:00"),
            visit_segment("place-b", "UNKNOWN", "14:00", "15:00"),
        ],
        profile_places: Vec::new(),
    });

    assert_eq!(batch.places.len(), 2);
    assert_eq!(batch.visits.len(), 3);

    let home = batch
        .places
        .iter()
        .find(|place| place.google_place_id == "place-a")
        .unwrap();
    assert_eq!(home.kind, PlaceKind::Home);
}

#[test]
fn import_labels_a_place_from_the_profile_when_no_visit_says_so() {
    let batch = build_batch(RawTimeline {
        segments: vec![visit_segment("place-b", "UNKNOWN", "14:00", "15:00")],
        profile_places: vec![RawProfilePlace {
            lat_lng: "48.853000°, 2.349900°".to_owned(),
            label: Some("WORK".to_owned()),
        }],
    });

    assert_eq!(batch.places[0].kind, PlaceKind::Work);
}

#[test]
fn import_skips_unusable_records_without_losing_the_rest() {
    let batch = build_batch(RawTimeline {
        segments: vec![
            RawSegment {
                start_time: "not a timestamp".to_owned(),
                end_time: "2024-12-01T10:00:00.000-03:00".to_owned(),
                kind: RawSegmentKind::Path(Vec::new()),
            },
            RawSegment {
                start_time: "2024-12-01T08:00:00.000-03:00".to_owned(),
                end_time: "2024-12-01T10:00:00.000-03:00".to_owned(),
                kind: RawSegmentKind::Path(vec![
                    RawTracePoint {
                        point: "broken".to_owned(),
                        time: "2024-12-01T08:10:00.000-03:00".to_owned(),
                    },
                    RawTracePoint {
                        point: "48.8583701°, 2.2944813°".to_owned(),
                        time: "2024-12-01T08:20:00.000-03:00".to_owned(),
                    },
                ]),
            },
        ],
        profile_places: Vec::new(),
    });

    assert_eq!(batch.skipped, 2);
    assert_eq!(batch.path_segments.len(), 1);
    assert_eq!(batch.path_segments[0].points.len(), 1);
}

#[test]
fn import_falls_back_to_the_straight_line_when_distance_is_missing() {
    let batch = build_batch(RawTimeline {
        segments: vec![RawSegment {
            start_time: "2024-12-01T08:00:00.000-03:00".to_owned(),
            end_time: "2024-12-01T09:00:00.000-03:00".to_owned(),
            kind: RawSegmentKind::Activity(RawActivity {
                start_lat_lng: "48.8566°, 2.3522°".to_owned(),
                end_lat_lng: "51.5074°, -0.1278°".to_owned(),
                distance_meters: f64::NAN,
                mode: "IN_BUS".to_owned(),
                probability: 0.9,
            }),
        }],
        profile_places: Vec::new(),
    });

    assert!(batch.activities[0].distance_meters > 300_000.0);
}

#[test]
fn a_day_summary_aggregates_distance_modes_and_the_longest_stay_away_from_home() {
    let places = [
        place("home", PlaceKind::Home),
        place("gym", PlaceKind::Unknown),
    ];
    let visits = vec![
        (visit(&places[0], "08:00", "12:00"), places[0].clone()),
        (visit(&places[1], "13:00", "15:00"), places[1].clone()),
    ];
    let activities = vec![
        activity("12:00", "12:30", 4_000.0, TravelMode::Walking),
        activity("15:00", "15:20", 9_000.0, TravelMode::Bus),
    ];

    let summaries = summarise_days(&activities, &visits);

    assert_eq!(summaries.len(), 1);
    let summary = &summaries[0];
    assert_eq!(summary.date.to_iso(), "2024-12-01");
    assert_eq!(summary.trip_count, 2);
    assert_eq!(summary.visit_count, 2);
    assert!((summary.distance_meters - 13_000.0).abs() < 1e-6);
    assert_eq!(summary.modes.len(), 2);
    assert_eq!(summary.highlight_place.as_deref(), Some("gym"));
    assert_eq!(
        summary.first_activity_at.unwrap().to_rfc3339(),
        instant("08:00").to_rfc3339()
    );
    assert_eq!(
        summary.last_activity_at.unwrap().to_rfc3339(),
        instant("15:20").to_rfc3339()
    );
}

#[test]
fn a_trace_splits_where_the_travel_mode_changes() {
    let segment = PathSegment {
        id: DomainId::new("segment-1").unwrap(),
        range: TimeRange::new(instant("08:00"), instant("09:00")).unwrap(),
        local_date: LocalDate::parse("2024-12-01").unwrap(),
        points: vec![
            trace_point("08:05", 0.80, 0.12),
            trace_point("08:15", 0.81, 0.11),
            trace_point("08:45", 0.85, 0.05),
            trace_point("08:55", 0.86, 0.04),
        ],
    };
    let activities = vec![
        activity("08:00", "08:20", 1_000.0, TravelMode::Walking),
        activity("08:40", "09:00", 8_000.0, TravelMode::Bus),
    ];

    let lines = build_trace(&[segment], &activities, &[]);

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].mode, Some(TravelMode::Walking));
    assert_eq!(lines[1].mode, Some(TravelMode::Bus));
    assert_eq!(
        lines[0].points.last().unwrap().coordinate,
        lines[1].points.first().unwrap().coordinate
    );
}

#[test]
fn a_trace_can_be_filtered_down_to_the_requested_modes() {
    let segment = PathSegment {
        id: DomainId::new("segment-1").unwrap(),
        range: TimeRange::new(instant("08:00"), instant("09:00")).unwrap(),
        local_date: LocalDate::parse("2024-12-01").unwrap(),
        points: vec![
            trace_point("08:05", 0.80, 0.12),
            trace_point("08:15", 0.81, 0.11),
            trace_point("08:45", 0.85, 0.05),
            trace_point("08:55", 0.86, 0.04),
        ],
    };
    let activities = vec![
        activity("08:00", "08:20", 1_000.0, TravelMode::Walking),
        activity("08:40", "09:00", 8_000.0, TravelMode::Bus),
    ];

    let lines = build_trace(&[segment], &activities, &[TravelMode::Bus]);

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].mode, Some(TravelMode::Bus));
}

#[tokio::test]
async fn import_writes_the_batch_and_reports_what_it_read() {
    let writer = FakeWriter::default();
    let source = FakeSource {
        raw: RawTimeline {
            segments: vec![
                visit_segment("place-a", "INFERRED_HOME", "08:00", "10:00"),
                RawSegment {
                    start_time: "2024-12-01T10:00:00.000-03:00".to_owned(),
                    end_time: "2024-12-01T10:30:00.000-03:00".to_owned(),
                    kind: RawSegmentKind::Path(vec![
                        RawTracePoint {
                            point: "0.80°, 0.12°".to_owned(),
                            time: "2024-12-01T10:05:00.000-03:00".to_owned(),
                        },
                        RawTracePoint {
                            point: "0.81°, 0.11°".to_owned(),
                            time: "2024-12-01T10:15:00.000-03:00".to_owned(),
                        },
                    ]),
                },
            ],
            profile_places: Vec::new(),
        },
    };

    let report = ImportTimeline::new(source, writer.clone())
        .execute()
        .await
        .unwrap();

    assert_eq!(report.places, 1);
    assert_eq!(report.visits, 1);
    assert_eq!(report.path_segments, 1);
    assert_eq!(report.trace_points, 2);
    assert_eq!(report.segments, 2);
    assert_eq!(writer.batches.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn resolving_names_carries_on_after_a_failure() {
    let writer = FakeWriter::with_pending(&["place-a", "place-b", "place-c"]);
    let resolver = FlakyResolver;

    let report = ResolvePlaceNames::new(writer.clone(), resolver)
        .execute()
        .await
        .unwrap();

    assert_eq!(report.pending, 3);
    assert_eq!(report.resolved, 2);
    assert_eq!(report.failed, 1);
    assert_eq!(writer.saved.lock().unwrap().len(), 2);
}

fn instant(time: &str) -> Instant {
    Instant::from_rfc3339(&format!("2024-12-01T{time}:00.000-03:00")).unwrap()
}

fn trace_point(time: &str, latitude: f64, longitude: f64) -> TracePoint {
    TracePoint {
        at: instant(time),
        coordinate: Coordinate::new(latitude, longitude).unwrap(),
    }
}

fn visit_segment(place_id: &str, semantic_type: &str, start: &str, end: &str) -> RawSegment {
    RawSegment {
        start_time: format!("2024-12-01T{start}:00.000-03:00"),
        end_time: format!("2024-12-01T{end}:00.000-03:00"),
        kind: RawSegmentKind::Visit(RawVisit {
            place_id: place_id.to_owned(),
            lat_lng: "48.853000°, 2.349900°".to_owned(),
            semantic_type: semantic_type.to_owned(),
            probability: 0.8,
        }),
    }
}

fn place(label: &str, kind: PlaceKind) -> Place {
    Place {
        id: DomainId::new(label).unwrap(),
        google_place_id: label.to_owned(),
        coordinate: Coordinate::new(0.86, -0.07).unwrap(),
        kind,
        display_name: Some(label.to_owned()),
    }
}

fn visit(place: &Place, start: &str, end: &str) -> Visit {
    Visit {
        id: DomainId::new(format!("visit-{start}")).unwrap(),
        place_id: place.id.clone(),
        range: TimeRange::new(instant(start), instant(end)).unwrap(),
        local_date: LocalDate::parse("2024-12-01").unwrap(),
        probability: 0.9,
    }
}

fn activity(start: &str, end: &str, distance_meters: f64, mode: TravelMode) -> Activity {
    Activity {
        id: DomainId::new(format!("activity-{start}")).unwrap(),
        range: TimeRange::new(instant(start), instant(end)).unwrap(),
        local_date: LocalDate::parse("2024-12-01").unwrap(),
        start: Coordinate::new(0.80, 0.12).unwrap(),
        end: Coordinate::new(0.86, 0.04).unwrap(),
        distance_meters,
        mode,
        probability: 0.95,
    }
}

struct FakeSource {
    raw: RawTimeline,
}

#[async_trait]
impl TimelineSource for FakeSource {
    async fn load(&self) -> Result<RawTimeline, AppError> {
        Ok(self.raw.clone())
    }

    fn describe(&self) -> String {
        "fake".to_owned()
    }
}

#[derive(Clone, Default)]
struct FakeWriter {
    batches: Arc<Mutex<Vec<ImportBatch>>>,
    pending: Arc<Mutex<Vec<PendingPlace>>>,
    saved: Arc<Mutex<Vec<(String, String)>>>,
}

impl FakeWriter {
    fn with_pending(ids: &[&str]) -> Self {
        let writer = Self::default();
        *writer.pending.lock().unwrap() = ids
            .iter()
            .map(|id| PendingPlace {
                id: (*id).to_owned(),
                coordinate: Coordinate::new(0.86, -0.07).unwrap(),
            })
            .collect();
        writer
    }
}

#[async_trait]
impl TimelineWriter for FakeWriter {
    async fn replace_all(&self, batch: ImportBatch, _source: &str) -> Result<(), AppError> {
        self.batches.lock().unwrap().push(batch);
        Ok(())
    }

    async fn places_without_name(&self) -> Result<Vec<PendingPlace>, AppError> {
        Ok(self.pending.lock().unwrap().clone())
    }

    async fn save_place_name(&self, place_id: &str, display_name: &str) -> Result<(), AppError> {
        self.saved
            .lock()
            .unwrap()
            .push((place_id.to_owned(), display_name.to_owned()));
        Ok(())
    }
}

struct FlakyResolver;

#[async_trait]
impl PlaceNameResolver for FlakyResolver {
    async fn resolve(&self, coordinate: Coordinate) -> Result<Option<String>, AppError> {
        let _ = coordinate;
        static ATTEMPTS: Mutex<u32> = Mutex::new(0);
        let mut attempts = ATTEMPTS.lock().unwrap();
        *attempts += 1;

        match *attempts {
            2 => Err(AppError::Internal("nominatim is unreachable".to_owned())),
            other => Ok(Some(format!("place {other}"))),
        }
    }
}
