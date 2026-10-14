import { useCallback, useLayoutEffect, useMemo, useRef, useState } from 'react'

import type { DailyDistance } from '../../api/types'
import { addDays, clampDate, daysBetween, formatDayShort, formatMonth } from '../../lib/format'

interface Props {
  series: DailyDistance[]
  first: string
  last: string
  from: string
  to: string
  onChange: (range: { from: string; to: string }) => void
}

const PRESETS = [
  { label: 'Tudo', days: null },
  { label: '12 m', days: 365 },
  { label: '90 d', days: 90 },
  { label: '30 d', days: 30 },
] as const

export function Seismograph({ series, first, last, from, to, onChange }: Props) {
  const plotRef = useRef<HTMLDivElement>(null)
  const [size, setSize] = useState({ width: 0, height: 0 })
  const [drag, setDrag] = useState<{ anchor: string } | null>(null)
  const [hover, setHover] = useState<string | null>(null)

  useLayoutEffect(() => {
    const element = plotRef.current
    if (!element) return
    const observer = new ResizeObserver(([entry]) => {
      setSize({ width: entry.contentRect.width, height: entry.contentRect.height })
    })
    observer.observe(element)
    return () => observer.disconnect()
  }, [])

  const totalDays = Math.max(1, daysBetween(first, last) + 1)
  const byDate = useMemo(() => {
    const map = new Map<string, number>()
    for (const day of series) map.set(day.date, day.distanceMeters)
    return map
  }, [series])

  const peak = useMemo(
    () => Math.max(1, ...series.map((day) => day.distanceMeters)),
    [series],
  )

  const xOf = useCallback(
    (date: string) => (daysBetween(first, date) / totalDays) * size.width,
    [first, totalDays, size.width],
  )

  const dateAt = useCallback(
    (clientX: number) => {
      const rect = plotRef.current?.getBoundingClientRect()
      if (!rect || rect.width === 0) return first
      const ratio = (clientX - rect.left) / rect.width
      const offset = Math.floor(ratio * totalDays)
      return clampDate(addDays(first, offset), first, last)
    },
    [first, last, totalDays],
  )

  const ridge = useMemo(() => {
    if (size.width === 0 || size.height === 0) return { all: '', selected: '' }
    const centre = size.height / 2
    const amplitude = centre - 10
    const step = size.width / totalDays
    const top: string[] = []
    const bottom: string[] = []
    const selectedTop: string[] = []
    const selectedBottom: string[] = []

    for (let index = 0; index < totalDays; index += 1) {
      const date = addDays(first, index)
      const distance = byDate.get(date) ?? 0
      const height = Math.sqrt(distance / peak) * amplitude
      const x = index * step
      top.push(`${x.toFixed(2)},${(centre - height).toFixed(2)}`)
      bottom.unshift(`${x.toFixed(2)},${(centre + height).toFixed(2)}`)

      if (date >= from && date <= to) {
        selectedTop.push(`${x.toFixed(2)},${(centre - height).toFixed(2)}`)
        selectedBottom.unshift(`${x.toFixed(2)},${(centre + height).toFixed(2)}`)
      }
    }

    return {
      all: `${top.join(' ')} ${bottom.join(' ')}`,
      selected: selectedTop.length ? `${selectedTop.join(' ')} ${selectedBottom.join(' ')}` : '',
    }
  }, [byDate, first, from, peak, size.height, size.width, to, totalDays])

  const months = useMemo(() => {
    const marks: { date: string; label: string; year: boolean }[] = []
    let cursor = `${first.slice(0, 7)}-01`
    while (cursor <= last) {
      if (cursor >= first) {
        const january = cursor.slice(5, 7) === '01'
        marks.push({
          date: cursor,
          label: january ? cursor.slice(0, 4) : formatMonth(cursor),
          year: january,
        })
      }
      const [year, month] = cursor.split('-').map(Number)
      cursor = month === 12 ? `${year + 1}-01-01` : `${year}-${`${month + 1}`.padStart(2, '0')}-01`
    }
    const minimumGap = 34
    const spaced: typeof marks = []
    for (const mark of marks) {
      const previous = spaced[spaced.length - 1]
      if (!previous) {
        spaced.push(mark)
        continue
      }
      if (xOf(mark.date) - xOf(previous.date) >= minimumGap) {
        spaced.push(mark)
      } else if (mark.year) {
        spaced[spaced.length - 1] = mark
      }
    }
    return spaced
  }, [first, last, xOf])

  const beginDrag = (event: React.PointerEvent) => {
    const anchor = dateAt(event.clientX)
    event.currentTarget.setPointerCapture(event.pointerId)
    setDrag({ anchor })
    onChange({ from: anchor, to: anchor })
  }

  const moveDrag = (event: React.PointerEvent) => {
    const date = dateAt(event.clientX)
    setHover(date)
    if (!drag) return
    onChange(
      date < drag.anchor ? { from: date, to: drag.anchor } : { from: drag.anchor, to: date },
    )
  }

  const applyPreset = (days: number | null) => {
    if (days === null) return onChange({ from: first, to: last })
    onChange({ from: clampDate(addDays(last, -days + 1), first, last), to: last })
  }

  const selectionWidth = Math.max(2, xOf(to) - xOf(from) + size.width / totalDays)

  return (
    <section className="seismo" aria-label="Linha do tempo completa">
      <header className="seismo__head">
        <span className="eyebrow">Sismógrafo · {totalDays} dias</span>
        <span className="seismo__readout">
          {formatDayShort(from)} — {formatDayShort(to)}
          {hover && <span style={{ color: 'var(--ink-faint)' }}> · {formatDayShort(hover)}</span>}
        </span>
        <div className="seismo__presets">
          {PRESETS.map((preset) => (
            <button
              key={preset.label}
              type="button"
              className="chip"
              onClick={() => applyPreset(preset.days)}
            >
              {preset.label}
            </button>
          ))}
        </div>
      </header>

      <div
        ref={plotRef}
        className="seismo__plot"
        onPointerDown={beginDrag}
        onPointerMove={moveDrag}
        onPointerUp={() => setDrag(null)}
        onPointerLeave={() => setHover(null)}
      >
        <svg className="seismo__svg" role="presentation">
          <line
            className="seismo__axis"
            x1={0}
            x2={size.width}
            y1={size.height / 2}
            y2={size.height / 2}
          />

          {ridge.all && <polygon className="seismo__ridge" points={ridge.all} />}
          {ridge.selected && (
            <polygon className="seismo__ridge seismo__ridge--on" points={ridge.selected} />
          )}

          {months.map((month) => (
            <g key={month.date}>
              <line
                className="seismo__axis"
                x1={xOf(month.date)}
                x2={xOf(month.date)}
                y1={size.height - 12}
                y2={size.height}
              />
              <text
                className={`seismo__month${month.year ? ' seismo__year' : ''}`}
                x={xOf(month.date) + 3}
                y={size.height - 3}
              >
                {month.label}
              </text>
            </g>
          ))}

          <rect
            className="seismo__brush"
            x={xOf(from)}
            y={0}
            width={selectionWidth}
            height={size.height}
            rx={1}
          />
          <rect className="seismo__handle" x={xOf(from) - 1} y={0} width={2} height={size.height} />
          <rect
            className="seismo__handle"
            x={xOf(from) + selectionWidth - 1}
            y={0}
            width={2}
            height={size.height}
          />
        </svg>
      </div>
    </section>
  )
}
