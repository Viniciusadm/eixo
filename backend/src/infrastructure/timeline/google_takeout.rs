use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::Deserialize;

use crate::application::timeline::{
    RawActivity, RawProfilePlace, RawSegment, RawSegmentKind, RawTimeline, RawTracePoint, RawVisit,
    TimelineSource,
};
use crate::errors::AppError;

pub struct GoogleTakeoutSource {
    path: PathBuf,
}

impl GoogleTakeoutSource {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

#[async_trait]
impl TimelineSource for GoogleTakeoutSource {
    async fn load(&self) -> Result<RawTimeline, AppError> {
        let bytes = tokio::fs::read(&self.path).await.map_err(|error| {
            AppError::Configuration(format!("could not read {}: {error}", self.path.display()))
        })?;
        let export: Export = serde_json::from_slice(&bytes).map_err(|error| {
            AppError::Validation(format!(
                "{} is not a Google Maps Timeline export: {error}",
                self.path.display()
            ))
        })?;

        Ok(RawTimeline {
            segments: export
                .semantic_segments
                .into_iter()
                .filter_map(into_segment)
                .collect(),
            profile_places: export
                .user_location_profile
                .map(|profile| {
                    profile
                        .frequent_places
                        .into_iter()
                        .map(|place| RawProfilePlace {
                            lat_lng: place.place_location,
                            label: place.label,
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    fn describe(&self) -> String {
        self.path.display().to_string()
    }
}

fn into_segment(segment: Segment) -> Option<RawSegment> {
    let start_time = segment.start_time?;
    let end_time = segment.end_time?;

    let kind = if let Some(visit) = segment.visit {
        let candidate = visit.top_candidate?;
        RawSegmentKind::Visit(RawVisit {
            place_id: candidate.place_id.unwrap_or_default(),
            lat_lng: candidate.place_location?.lat_lng?,
            semantic_type: candidate.semantic_type.unwrap_or_default(),
            probability: visit.probability.unwrap_or_default(),
        })
    } else if let Some(activity) = segment.activity {
        let candidate = activity.top_candidate?;
        RawSegmentKind::Activity(RawActivity {
            start_lat_lng: activity.start?.lat_lng?,
            end_lat_lng: activity.end?.lat_lng?,
            distance_meters: activity.distance_meters.unwrap_or(f64::NAN),
            mode: candidate.activity_type.unwrap_or_default(),
            probability: activity.probability.unwrap_or_default(),
        })
    } else if let Some(path) = segment.timeline_path {
        RawSegmentKind::Path(
            path.into_iter()
                .filter_map(|point| {
                    Some(RawTracePoint {
                        point: point.point?,
                        time: point.time?,
                    })
                })
                .collect(),
        )
    } else {
        return None;
    };

    Some(RawSegment {
        start_time,
        end_time,
        kind,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Export {
    #[serde(default)]
    semantic_segments: Vec<Segment>,
    #[serde(default)]
    user_location_profile: Option<UserLocationProfile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Segment {
    start_time: Option<String>,
    end_time: Option<String>,
    #[serde(default)]
    timeline_path: Option<Vec<PathPoint>>,
    #[serde(default)]
    visit: Option<VisitSegment>,
    #[serde(default)]
    activity: Option<ActivitySegment>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PathPoint {
    point: Option<String>,
    time: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VisitSegment {
    probability: Option<f64>,
    top_candidate: Option<VisitCandidate>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VisitCandidate {
    place_id: Option<String>,
    semantic_type: Option<String>,
    place_location: Option<LatLng>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivitySegment {
    start: Option<LatLng>,
    end: Option<LatLng>,
    distance_meters: Option<f64>,
    probability: Option<f64>,
    top_candidate: Option<ActivityCandidate>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityCandidate {
    #[serde(rename = "type")]
    activity_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LatLng {
    #[serde(alias = "latLng", alias = "LatLng")]
    lat_lng: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserLocationProfile {
    #[serde(default)]
    frequent_places: Vec<FrequentPlace>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FrequentPlace {
    place_location: String,
    label: Option<String>,
}
