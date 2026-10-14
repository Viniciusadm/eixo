import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import type { DayEntry, TraceCollection } from '../../api/types'

const EMPTY: TraceCollection = { type: 'FeatureCollection', features: [] }

export const SPEEDS = [60, 240, 900] as const

export interface Playback {
  playing: boolean
  clock: number | null
  speed: number
  drawn: TraceCollection | null
  head: [number, number] | null
  activeEntryId: string | null
  available: boolean
  toggle: () => void
  setSpeed: (speed: number) => void
  reset: () => void
}

export function usePlayback(trace: TraceCollection | undefined, entries: DayEntry[]): Playback {
  const [playing, setPlaying] = useState(false)
  const [clock, setClock] = useState<number | null>(null)
  const [speed, setSpeed] = useState<number>(SPEEDS[1])
  const frame = useRef<number>(0)
  const previous = useRef<number>(0)

  const span = useMemo(() => {
    const stamps = (trace?.features ?? []).flatMap((feature) => feature.properties.timestamps)
    if (stamps.length < 2) return null
    return { start: Math.min(...stamps), end: Math.max(...stamps) }
  }, [trace])

  const traceKey = trace?.features.map((feature) => feature.id).join('|') ?? ''
  const [lastTraceKey, setLastTraceKey] = useState(traceKey)
  if (traceKey !== lastTraceKey) {
    setLastTraceKey(traceKey)
    setPlaying(false)
    setClock(null)
  }

  const reset = useCallback(() => {
    setPlaying(false)
    setClock(null)
  }, [])

  useEffect(() => {
    if (!playing || !span) return

    previous.current = performance.now()
    const tick = (now: number) => {
      const elapsed = (now - previous.current) / 1000
      previous.current = now
      setClock((current) => {
        const next = (current ?? span.start) + elapsed * speed
        if (next >= span.end) {
          setPlaying(false)
          return span.end
        }
        return next
      })
      frame.current = requestAnimationFrame(tick)
    }

    frame.current = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(frame.current)
  }, [playing, span, speed])

  const toggle = useCallback(() => {
    if (!span) return
    setPlaying((current) => {
      if (!current) setClock((value) => (value === null || value >= span.end ? span.start : value))
      return !current
    })
  }, [span])

  const drawn = useMemo(() => {
    if (clock === null || !trace) return null
    const features = trace.features
      .map((feature) => clipFeature(feature, clock))
      .filter((feature): feature is TraceCollection['features'][number] => feature !== null)
    return features.length ? { type: 'FeatureCollection' as const, features } : EMPTY
  }, [clock, trace])

  const head = useMemo(() => {
    if (clock === null || !drawn) return null
    let latest: { at: number; position: [number, number] } | null = null
    for (const feature of drawn.features) {
      const at = feature.properties.timestamps.at(-1)
      const position = feature.geometry.coordinates.at(-1)
      if (at === undefined || !position) continue
      if (!latest || at > latest.at) latest = { at, position }
    }
    return latest?.position ?? null
  }, [clock, drawn])

  const activeEntryId = useMemo(() => {
    if (clock === null) return null
    const millis = clock * 1000
    const current = entries.find(
      (entry) =>
        new Date(entry.startedAt).getTime() <= millis && millis <= new Date(entry.endedAt).getTime(),
    )
    return current?.id ?? null
  }, [clock, entries])

  return {
    playing,
    clock,
    speed,
    drawn,
    head,
    activeEntryId,
    available: span !== null,
    toggle,
    setSpeed,
    reset,
  }
}

function clipFeature(
  feature: TraceCollection['features'][number],
  clock: number,
): TraceCollection['features'][number] | null {
  const { timestamps } = feature.properties
  const coordinates = feature.geometry.coordinates

  if (timestamps[0] > clock) return null
  if (timestamps[timestamps.length - 1] <= clock) return feature

  const kept: [number, number][] = []
  const keptStamps: number[] = []
  for (let index = 0; index < coordinates.length; index += 1) {
    if (timestamps[index] <= clock) {
      kept.push(coordinates[index])
      keptStamps.push(timestamps[index])
      continue
    }
    const previousStamp = timestamps[index - 1]
    const ratio = (clock - previousStamp) / (timestamps[index] - previousStamp)
    const [fromLng, fromLat] = coordinates[index - 1]
    const [toLng, toLat] = coordinates[index]
    kept.push([fromLng + (toLng - fromLng) * ratio, fromLat + (toLat - fromLat) * ratio])
    keptStamps.push(clock)
    break
  }

  if (kept.length < 2) return null
  return {
    ...feature,
    properties: { ...feature.properties, timestamps: keptStamps },
    geometry: { type: 'LineString', coordinates: kept },
  }
}
