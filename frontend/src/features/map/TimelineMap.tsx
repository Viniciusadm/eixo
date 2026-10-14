import { useEffect, useMemo, useRef } from 'react'
import Map, { Layer, Source, type MapRef } from 'react-map-gl/maplibre'
import type { LngLatBoundsLike } from 'maplibre-gl'

import type { DayEntry, PlaceRanking, TraceCollection } from '../../api/types'
import { modeColorExpression, UNTRACKED_COLOR } from '../../lib/modes'

const MAP_STYLE =
  import.meta.env.VITE_MAP_STYLE ??
  'https://basemaps.cartocdn.com/gl/dark-matter-gl-style/style.json'

const INITIAL_CENTER: [number, number] = [0, 20]

const EMPTY: TraceCollection = { type: 'FeatureCollection', features: [] }

interface Props {
  ghost: TraceCollection
  active: TraceCollection
  drawn: TraceCollection | null
  places: PlaceRanking[]
  entries: DayEntry[]
  focus: DayEntry | null
  head: [number, number] | null
}

export function TimelineMap({ ghost, active, drawn, places, entries, focus, head }: Props) {
  const mapRef = useRef<MapRef>(null)

  const stays = useMemo(
    () => ({
      type: 'FeatureCollection' as const,
      features: places.map((place) => ({
        type: 'Feature' as const,
        id: place.placeId,
        properties: {
          label: place.label,
          kind: place.kind,
          hours: place.totalSeconds / 3600,
          weight: Math.sqrt(place.totalSeconds / 3600),
        },
        geometry: { type: 'Point' as const, coordinates: place.coordinate },
      })),
    }),
    [places],
  )

  const dayStops = useMemo(
    () => ({
      type: 'FeatureCollection' as const,
      features: entries
        .filter((entry): entry is Extract<DayEntry, { type: 'stay' }> => entry.type === 'stay')
        .map((entry) => ({
          type: 'Feature' as const,
          id: entry.id,
          properties: { label: entry.label, kind: entry.kind },
          geometry: { type: 'Point' as const, coordinates: entry.coordinate },
        })),
    }),
    [entries],
  )

  const marker = useMemo(
    () =>
      head
        ? {
            type: 'FeatureCollection' as const,
            features: [
              {
                type: 'Feature' as const,
                properties: {},
                geometry: { type: 'Point' as const, coordinates: head },
              },
            ],
          }
        : EMPTY,
    [head],
  )

  const bounds = useMemo(() => boundsOf(active.features.length ? active : ghost), [active, ghost])

  useEffect(() => {
    if (!bounds || !mapRef.current) return
    mapRef.current.fitBounds(bounds, {
      padding: { top: 80, bottom: 80, left: 80, right: 80 },
      duration: prefersReducedMotion() ? 0 : 900,
      maxZoom: 15,
    })
  }, [bounds])

  useEffect(() => {
    if (!focus || !mapRef.current) return
    const target = focus.type === 'stay' ? focus.coordinate : focus.start
    mapRef.current.easeTo({
      center: target,
      zoom: Math.max(mapRef.current.getZoom(), 14),
      duration: prefersReducedMotion() ? 0 : 700,
    })
  }, [focus])

  return (
    <Map
      ref={mapRef}
      initialViewState={{ longitude: INITIAL_CENTER[0], latitude: INITIAL_CENTER[1], zoom: 1.5 }}
      mapStyle={MAP_STYLE}
      attributionControl={{ compact: true }}
      style={{ width: '100%', height: '100%' }}
    >
      <Source id="ghost" type="geojson" data={ghost}>
        <Layer
          id="ghost-line"
          type="line"
          layout={{ 'line-cap': 'round', 'line-join': 'round' }}
          paint={{ 'line-color': UNTRACKED_COLOR, 'line-width': 1.1, 'line-opacity': 0.34 }}
        />
      </Source>

      <Source id="active" type="geojson" data={active}>
        <Layer
          id="active-glow"
          type="line"
          layout={{ 'line-cap': 'round', 'line-join': 'round' }}
          paint={{
            'line-color': modeColorExpression() as never,
            'line-width': ['interpolate', ['linear'], ['zoom'], 9, 5, 15, 14],
            'line-opacity': 0.14,
            'line-blur': 6,
          }}
        />
        <Layer
          id="active-line"
          type="line"
          layout={{ 'line-cap': 'round', 'line-join': 'round' }}
          paint={{
            'line-color': modeColorExpression() as never,
            'line-width': ['interpolate', ['linear'], ['zoom'], 9, 1.4, 15, 3.4],
            'line-opacity': drawn === null ? 0.92 : 0.22,
          }}
        />
      </Source>

      <Source id="drawn" type="geojson" data={drawn ?? EMPTY}>
        <Layer
          id="drawn-glow"
          type="line"
          layout={{ 'line-cap': 'round', 'line-join': 'round' }}
          paint={{
            'line-color': modeColorExpression() as never,
            'line-width': ['interpolate', ['linear'], ['zoom'], 9, 7, 15, 18],
            'line-opacity': 0.2,
            'line-blur': 8,
          }}
        />
        <Layer
          id="drawn-line"
          type="line"
          layout={{ 'line-cap': 'round', 'line-join': 'round' }}
          paint={{
            'line-color': modeColorExpression() as never,
            'line-width': ['interpolate', ['linear'], ['zoom'], 9, 2, 15, 4.4],
            'line-opacity': 1,
          }}
        />
      </Source>

      <Source id="stays" type="geojson" data={stays}>
        <Layer
          id="stays-dot"
          type="circle"
          paint={{
            'circle-radius': ['interpolate', ['linear'], ['get', 'weight'], 0, 2.5, 80, 16],
            'circle-color': [
              'match',
              ['get', 'kind'],
              'HOME',
              '#f4f0e6',
              'WORK',
              '#b9c9d2',
              '#6d8996',
            ],
            'circle-opacity': 0.16,
            'circle-stroke-width': 1,
            'circle-stroke-color': [
              'match',
              ['get', 'kind'],
              'HOME',
              '#f4f0e6',
              'WORK',
              '#b9c9d2',
              '#6d8996',
            ],
            'circle-stroke-opacity': 0.5,
          }}
        />
      </Source>

      <Source id="day-stops" type="geojson" data={dayStops}>
        <Layer
          id="day-stops-dot"
          type="circle"
          paint={{
            'circle-radius': 4,
            'circle-color': '#090e11',
            'circle-stroke-width': 1.5,
            'circle-stroke-color': [
              'match',
              ['get', 'kind'],
              'HOME',
              '#f4f0e6',
              'WORK',
              '#b9c9d2',
              '#ffffff',
            ],
          }}
        />
        <Layer
          id="day-stops-label"
          type="symbol"
          layout={{
            'text-field': ['get', 'label'],
            'text-font': ['Open Sans Regular', 'Arial Unicode MS Regular'],
            'text-size': 10,
            'text-offset': [0, 1.1],
            'text-anchor': 'top',
            'text-max-width': 12,
            'text-allow-overlap': false,
          }}
          paint={{
            'text-color': '#c4d2d9',
            'text-halo-color': '#090e11',
            'text-halo-width': 1.4,
          }}
        />
      </Source>

      <Source id="head" type="geojson" data={marker}>
        <Layer
          id="head-halo"
          type="circle"
          paint={{ 'circle-radius': 13, 'circle-color': '#ffffff', 'circle-opacity': 0.1 }}
        />
        <Layer
          id="head-dot"
          type="circle"
          paint={{
            'circle-radius': 5,
            'circle-color': '#ffffff',
            'circle-stroke-width': 2,
            'circle-stroke-color': '#090e11',
          }}
        />
      </Source>
    </Map>
  )
}

function prefersReducedMotion(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches
}

function boundsOf(collection: TraceCollection): LngLatBoundsLike | null {
  let west = Infinity
  let south = Infinity
  let east = -Infinity
  let north = -Infinity

  for (const feature of collection.features) {
    for (const [longitude, latitude] of feature.geometry.coordinates) {
      west = Math.min(west, longitude)
      east = Math.max(east, longitude)
      south = Math.min(south, latitude)
      north = Math.max(north, latitude)
    }
  }

  if (!Number.isFinite(west)) return null
  return [
    [west, south],
    [east, north],
  ]
}
