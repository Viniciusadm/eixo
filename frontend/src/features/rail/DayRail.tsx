import { useEffect, useRef } from 'react'

import type { DayDetail, DaySummary, DayEntry } from '../../api/types'
import { formatDayRail, formatDistance, formatDuration, formatSpan, formatTime } from '../../lib/format'
import { modeStyle, PLACE_KIND } from '../../lib/modes'

interface Props {
  days: DaySummary[]
  openDate: string | null
  detail: DayDetail | undefined
  activeEntryId: string | null
  onToggleDay: (date: string) => void
  onFocusEntry: (entry: DayEntry) => void
}

export function DayRail({
  days,
  openDate,
  detail,
  activeEntryId,
  onToggleDay,
  onFocusEntry,
}: Props) {
  const activeRef = useRef<HTMLLIElement>(null)

  useEffect(() => {
    activeRef.current?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
  }, [activeEntryId])

  if (days.length === 0) {
    return (
      <div className="rail__empty">
        <p className="eyebrow">Intervalo vazio</p>
        <p>Nenhum deslocamento registrado aqui. Arraste o sismógrafo para outro período.</p>
      </div>
    )
  }

  const peak = Math.max(1, ...days.map((day) => day.distanceMeters))

  return (
    <ol className="rail__list">
      {days.map((day) => {
        const open = day.date === openDate
        return (
          <li key={day.date} className="day">
            <button
              type="button"
              className="day__head"
              onClick={() => onToggleDay(day.date)}
              aria-expanded={open}
            >
              <span className="day__node" />
              <span className="day__date">{formatDayRail(day.date)}</span>
              <span className="day__gauge">
                <span
                  style={{
                    width: `${Math.sqrt(day.distanceMeters / peak) * 100}%`,
                    background: modeStyle(day.modes[0]).color,
                  }}
                />
              </span>
              <span className="day__distance">{formatDistance(day.distanceMeters)}</span>
            </button>

            {open && detail?.date === day.date && (
              <div className="day__body">
                <ul className="day__entries">
                  {detail.entries.map((entry) => (
                    <li key={entry.id} ref={entry.id === activeEntryId ? activeRef : undefined}>
                      <EntryButton
                        entry={entry}
                        active={entry.id === activeEntryId}
                        onFocus={onFocusEntry}
                      />
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </li>
        )
      })}
    </ol>
  )
}

interface EntryProps {
  entry: DayEntry
  active: boolean
  onFocus: (entry: DayEntry) => void
}

function EntryButton({ entry, active, onFocus }: EntryProps) {
  if (entry.type === 'stay') {
    const kind = PLACE_KIND[entry.kind]
    const accent = entry.kind === 'HOME' ? '#f4f0e6' : entry.kind === 'WORK' ? '#b9c9d2' : '#6d8996'

    return (
      <button
        type="button"
        className={`entry entry--stay${active ? ' entry--active' : ''}`}
        onClick={() => onFocus(entry)}
      >
        <span className="entry__node" style={{ background: accent }} />
        <span className="entry__label">
          {kind.short && (
            <span className="entry__badge" style={{ background: accent }}>
              {kind.short}
            </span>
          )}
          {entry.label}
        </span>
        <span className="entry__meta">
          <span>{formatSpan(entry.startedAt, entry.endedAt)}</span>
          <span>{formatDuration(entry.durationSeconds)}</span>
        </span>
        <span
          className="entry__bar"
          style={{
            background: accent,
            width: `${Math.min(100, Math.sqrt(entry.durationSeconds / 43_200) * 100)}%`,
          }}
        />
      </button>
    )
  }

  const style = modeStyle(entry.mode)

  return (
    <button
      type="button"
      className={`entry entry--journey${active ? ' entry--active' : ''}`}
      onClick={() => onFocus(entry)}
    >
      <span className="entry__node" style={{ background: style.color }} />
      <span className="entry__journey-line">
        <span className="entry__mode" style={{ color: style.color }}>
          {style.label}
        </span>
        <span>{formatDistance(entry.distanceMeters)}</span>
        <span>{formatDuration(entry.durationSeconds)}</span>
        <span style={{ marginLeft: 'auto' }}>{formatTime(entry.startedAt)}</span>
      </span>
    </button>
  )
}
