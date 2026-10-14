# Eixo

Eixo is a viewer for a Google Maps Timeline export. It draws the recorded path on a map, colored by travel mode, and lets you move through the history day by day. This repository holds the API that imports the export into SQLite and the web interface that reads from it.

| Part | What it is |
| --- | --- |
| [`backend`](backend) | Rust API (axum + SQLite, hexagonal architecture) and the `import` binary that loads the export and resolves place names through Nominatim. Started from [rust-backend-template](https://github.com/Viniciusadm/rust-backend-template), with SQLite instead of MySQL and without the auth slice. Its own README documents the endpoints, the data model and the environment variables. |
| [`frontend`](frontend) | React + TypeScript app with MapLibre GL: the map, a day-by-day side rail, a "seismograph" of daily distance used to pick a date range, and playback of a single day. |

The export itself (`timeline.json`), the SQLite database and the place name cache in `backend/data/` hold personal location history and are ignored by git.

## Requirements

- Rust (edition 2021)
- Node.js **20.19+** or **22.12+** (Vite 8)
- A Google Maps Timeline export saved as `timeline.json` at the repository root

## Running locally

Import the export into SQLite and resolve the place names (the import replaces the whole database on each run; resolved names are cached in `backend/data/places-cache.json`):

```bash
cd backend
cp .env.example .env
cargo run --bin import -- ../timeline.json --geocode
```

Start the API on http://127.0.0.1:3001:

```bash
cargo run
```

In another terminal, start the interface on http://localhost:5173:

```bash
cd frontend
npm install
npm run dev
```

Set `NOMINATIM_USER_AGENT` in `backend/.env` before geocoding: the Nominatim usage policy asks for an identifiable `User-Agent` and at most one request per second, which the resolver respects. The base map is Carto Dark Matter (no API key) with OpenStreetMap data.

## Tests

```bash
cd backend
cargo test
```

`cargo test` covers the use cases with fakes and no database: coordinate parsing, travel mode mapping, local dates, distance, the import (deduplication, labels, skipped records), day summaries, trace splitting and place name resolution.
