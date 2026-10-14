export type TravelMode =
  | 'WALKING'
  | 'IN_PASSENGER_VEHICLE'
  | 'IN_SUBWAY'
  | 'IN_BUS'
  | 'CYCLING'
  | 'IN_TRAIN'
  | 'IN_TAXI'
  | (string & {})

export type PlaceKind = 'HOME' | 'WORK' | 'UNKNOWN'

export interface ModeTotal {
  mode: TravelMode
  tripCount: number
  distanceMeters: number
  durationSeconds: number
}

export interface DailyDistance {
  date: string
  distanceMeters: number
  tripCount: number
  visitCount: number
}

export interface Overview {
  firstDate: string | null
  lastDate: string | null
  trackedDays: number
  totalDistanceMeters: number
  visitCount: number
  placeCount: number
  tracePointCount: number
  modes: ModeTotal[]
  dailyDistance: DailyDistance[]
}

export interface DaySummary {
  date: string
  distanceMeters: number
  movingSeconds: number
  tripCount: number
  visitCount: number
  modes: TravelMode[]
  firstActivityAt: string | null
  lastActivityAt: string | null
  highlightPlace: string | null
}

export interface StayEntry {
  type: 'stay'
  id: string
  placeId: string
  label: string
  kind: PlaceKind
  coordinate: [number, number]
  startedAt: string
  endedAt: string
  durationSeconds: number
  probability: number
}

export interface JourneyEntry {
  type: 'journey'
  id: string
  mode: TravelMode
  startedAt: string
  endedAt: string
  durationSeconds: number
  distanceMeters: number
  start: [number, number]
  end: [number, number]
  probability: number
}

export type DayEntry = StayEntry | JourneyEntry

export interface TraceProperties {
  id: string
  date: string
  mode: TravelMode | null
  startedAt: string | null
  endedAt: string | null
  timestamps: number[]
}

export interface TraceFeature {
  type: 'Feature'
  id: string
  properties: TraceProperties
  geometry: { type: 'LineString'; coordinates: [number, number][] }
}

export interface TraceCollection {
  type: 'FeatureCollection'
  features: TraceFeature[]
}

export interface DayDetail {
  date: string
  summary: DaySummary
  entries: DayEntry[]
  trace: TraceCollection
}

export interface PlaceRanking {
  placeId: string
  label: string
  kind: PlaceKind
  coordinate: [number, number]
  visitCount: number
  totalSeconds: number
  firstSeen: string
  lastSeen: string
}

export interface RangeStats {
  from: string | null
  to: string | null
  distanceMeters: number
  movingSeconds: number
  trackedDays: number
  tripCount: number
  visitCount: number
  distinctPlaces: number
  modes: ModeTotal[]
  busiestDay: DailyDistance | null
}
