CREATE TABLE places (
  id TEXT PRIMARY KEY,
  google_place_id TEXT NOT NULL UNIQUE,
  lat REAL NOT NULL,
  lng REAL NOT NULL,
  kind TEXT NOT NULL,
  display_name TEXT,
  resolved_at TEXT
);

CREATE TABLE visits (
  id TEXT PRIMARY KEY,
  place_id TEXT NOT NULL REFERENCES places(id) ON DELETE CASCADE,
  started_at TEXT NOT NULL,
  ended_at TEXT NOT NULL,
  offset_minutes INTEGER NOT NULL,
  local_date TEXT NOT NULL,
  duration_seconds INTEGER NOT NULL,
  probability REAL NOT NULL
);

CREATE TABLE activities (
  id TEXT PRIMARY KEY,
  started_at TEXT NOT NULL,
  ended_at TEXT NOT NULL,
  offset_minutes INTEGER NOT NULL,
  local_date TEXT NOT NULL,
  start_lat REAL NOT NULL,
  start_lng REAL NOT NULL,
  end_lat REAL NOT NULL,
  end_lng REAL NOT NULL,
  duration_seconds INTEGER NOT NULL,
  distance_meters REAL NOT NULL,
  mode TEXT NOT NULL,
  probability REAL NOT NULL
);

CREATE TABLE path_segments (
  id TEXT PRIMARY KEY,
  started_at TEXT NOT NULL,
  ended_at TEXT NOT NULL,
  offset_minutes INTEGER NOT NULL,
  local_date TEXT NOT NULL
);

CREATE TABLE path_points (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  segment_id TEXT NOT NULL REFERENCES path_segments(id) ON DELETE CASCADE,
  recorded_at TEXT NOT NULL,
  offset_minutes INTEGER NOT NULL,
  local_date TEXT NOT NULL,
  lat REAL NOT NULL,
  lng REAL NOT NULL
);

CREATE TABLE import_runs (
  id TEXT PRIMARY KEY,
  source TEXT NOT NULL,
  imported_at TEXT NOT NULL,
  places INTEGER NOT NULL,
  visits INTEGER NOT NULL,
  activities INTEGER NOT NULL,
  path_segments INTEGER NOT NULL,
  trace_points INTEGER NOT NULL,
  skipped INTEGER NOT NULL
);

CREATE INDEX visits_local_date ON visits(local_date, started_at);
CREATE INDEX visits_place ON visits(place_id);
CREATE INDEX activities_local_date ON activities(local_date, started_at);
CREATE INDEX activities_mode ON activities(mode);
CREATE INDEX path_segments_local_date ON path_segments(local_date, started_at);
CREATE INDEX path_points_segment ON path_points(segment_id, recorded_at);
CREATE INDEX path_points_local_date ON path_points(local_date, recorded_at);
