# timeline-api

Backend do Eixo em **arquitetura hexagonal (Ports & Adapters)**, gerado a
partir do [`rust-backend-template`](https://github.com/Viniciusadm/rust-backend-template)
e adaptado: SQLite no lugar do MySQL e sem o slice de autenticação.

## Rodar

```bash
cp .env.example .env
cargo test                                        # casos de uso, sem banco
cargo run --bin import -- ../timeline.json        # popula o SQLite
cargo run --bin import -- ../timeline.json --geocode   # + nomes dos lugares
cargo run                                         # sobe a API
```

## Arquitetura

As dependências apontam **para dentro**: `api → application → domain`. A
infraestrutura implementa os traits (ports) definidos em `application` e é
injetada nos handlers por request.

```
src/
├── domain/            Regras e tipos puros (sem axum/sqlx/serde)
│   ├── value_objects.rs   Coordinate, Instant, LocalDate, TimeRange, TravelMode, PlaceKind
│   └── timeline/          Place, Visit, Activity, PathSegment, TracePoint
├── application/       Casos de uso + ports (traits)
│   └── timeline/
│       ├── ports.rs       TimelineRepository, TimelineWriter, TimelineSource, PlaceNameResolver
│       ├── commands.rs    DTOs de entrada/saída
│       └── use_cases.rs   TimelineQueries, ImportTimeline, ResolvePlaceNames
├── infrastructure/    Adaptadores concretos
│   ├── timeline/          GoogleTakeoutSource (lê o export)
│   ├── geocoding/         NominatimResolver + CachedPlaceNames
│   └── repositories/      SqliteTimelineRepository + mapper
├── database/          pool + transaction
├── config/            AppConfig::from_env()
├── errors/            AppError (enum único) + códigos estáveis
└── api/               Camada HTTP (axum)
```

A leitura pelo repositório é deliberadamente de baixo nível: o SQL faz contagem
e filtro, e a montagem de dias, trajetos e linhas desenháveis fica nos casos de
uso, onde é testável com fakes.

## Endpoints

| Método | Rota | Devolve |
|---|---|---|
| GET | `/health` | `ok` |
| GET | `/api/v1/timeline/overview` | período, totais, split por modal e a série de km por dia |
| GET | `/api/v1/timeline/days?from=&to=` | resumo de cada dia do intervalo |
| GET | `/api/v1/timeline/days/:date` | entradas cronológicas do dia + rota em GeoJSON |
| GET | `/api/v1/timeline/path?from=&to=&modes=` | `FeatureCollection` de `LineString` por modal |
| GET | `/api/v1/timeline/places?limit=` | lugares com nome, tempo total e nº de visitas |
| GET | `/api/v1/timeline/stats?from=&to=` | agregados do intervalo |
| POST | `/api/v1/timeline/places/resolve-names` | resolve os nomes que faltam via Nominatim |

O GeoJSON é montado no backend porque é o formato que o MapLibre consome
direto.

## Modelo de dados

`places` (deduplicado por `google_place_id`), `visits`, `activities`,
`path_segments` e `path_points`, mais `import_runs` com o histórico das
importações. Timestamps são gravados em UTC ISO-8601 de largura fixa, e cada
registro guarda também o `offset_minutes` e a `local_date` derivada do próprio
fuso do export — é essa data local, e não o fuso do servidor, que define a que
dia o registro pertence.

## Variáveis de ambiente

| Variável | Default | Descrição |
|---|---|---|
| `HTTP_HOST` | `127.0.0.1` | Host de bind |
| `HTTP_PORT` | `3001` | Porta |
| `DATABASE_URL` | `sqlite://data/timeline.db` | Banco SQLite (criado no primeiro boot) |
| `RUN_MIGRATIONS` | `true` | Rodar migrations no boot |
| `TIMELINE_JSON_PATH` | `../timeline.json` | Export usado pelo binário de import |
| `NOMINATIM_BASE_URL` | `https://nominatim.openstreetmap.org` | Serviço de reverse geocoding |
| `NOMINATIM_USER_AGENT` | `timeline-api/0.1 (self-hosted)` | Exigido pela política do Nominatim |
| `PLACES_CACHE_PATH` | `data/places-cache.json` | Cache dos nomes já resolvidos |
