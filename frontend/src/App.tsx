import { useMemo, useState } from 'react'

import type { DayEntry, TraceCollection } from './api/types'
import { Masthead } from './features/header/Masthead'
import { Legend } from './features/map/Legend'
import { TimelineMap } from './features/map/TimelineMap'
import { PlaybackControls } from './features/playback/PlaybackControls'
import { usePlayback } from './features/playback/usePlayback'
import { DayRail } from './features/rail/DayRail'
import { Seismograph } from './features/seismograph/Seismograph'
import { useDayDetail, useDays, useOverview, usePlaces, useTrace } from './hooks/useTimeline'
import { addDays, formatDayLong } from './lib/format'

const EMPTY_TRACE: TraceCollection = { type: 'FeatureCollection', features: [] }

export function App() {
  const overview = useOverview()
  const places = usePlaces(200)

  const [chosenRange, setChosenRange] = useState<{ from: string; to: string } | null>(null)
  const [openDate, setOpenDate] = useState<string | null>(null)
  const [modes, setModes] = useState<string[]>([])
  const [focus, setFocus] = useState<DayEntry | null>(null)

  const defaultRange = useMemo(() => {
    const first = overview.data?.firstDate
    const last = overview.data?.lastDate
    if (!first || !last) return null
    const from = addDays(last, -29)
    return { from: from < first ? first : from, to: last }
  }, [overview.data])

  const range = chosenRange ?? defaultRange

  const days = useDays(range)
  const trace = useTrace(range, modes)
  const detail = useDayDetail(openDate)
  const playback = usePlayback(detail.data?.trace, detail.data?.entries ?? [])

  const wholeHistory = useMemo(
    () =>
      overview.data?.firstDate && overview.data.lastDate
        ? { from: overview.data.firstDate, to: overview.data.lastDate }
        : null,
    [overview.data],
  )
  const ghost = useTrace(wholeHistory, [])

  const selectedDays = days.data ?? []
  const distanceInRange = selectedDays.reduce((total, day) => total + day.distanceMeters, 0)
  const placesInRange = selectedDays.reduce((total, day) => total + day.visitCount, 0)

  const activeTrace = detail.data?.trace ?? trace.data ?? EMPTY_TRACE

  if (overview.isPending) {
    return (
      <main className="shell shell--loading">
        <p className="eyebrow">Carregando o histórico</p>
      </main>
    )
  }

  if (overview.isError || !overview.data?.firstDate || !overview.data.lastDate || !range) {
    return (
      <main className="shell shell--error">
        <p className="eyebrow">Sem dados</p>
        <p>
          {overview.error instanceof Error
            ? overview.error.message
            : 'Rode o import antes de abrir o app:'}
        </p>
        <code className="numeral">cargo run --bin import -- ../timeline.json --geocode</code>
      </main>
    )
  }

  const handleToggleDay = (date: string) => {
    setOpenDate((current) => (current === date ? null : date))
    setFocus(null)
  }

  const handleRange = (next: { from: string; to: string }) => {
    setChosenRange(next)
    setOpenDate(null)
    setFocus(null)
  }

  const handleToggleMode = (mode: string) => {
    setModes((current) =>
      current.includes(mode) ? current.filter((value) => value !== mode) : [...current, mode],
    )
  }

  return (
    <main className="shell">
      <Masthead
        overview={overview.data}
        days={selectedDays.length}
        distanceMeters={distanceInRange}
        places={placesInRange}
      />

      <nav className="rail" aria-label="Dias do período selecionado">
        <div className="rail__head">
          <span className="eyebrow">
            {selectedDays.length} {selectedDays.length === 1 ? 'dia' : 'dias'} no recorte
          </span>
          {openDate && (
            <button type="button" className="chip" onClick={() => setOpenDate(null)}>
              fechar
            </button>
          )}
        </div>

        {days.isPending ? (
          <p className="rail__empty eyebrow">Carregando</p>
        ) : (
          <DayRail
            days={selectedDays}
            openDate={openDate}
            detail={detail.data}
            activeEntryId={playback.activeEntryId ?? focus?.id ?? null}
            onToggleDay={handleToggleDay}
            onFocusEntry={setFocus}
          />
        )}
      </nav>

      <section className="canvas">
        <div className="canvas__map">
          <TimelineMap
            ghost={ghost.data ?? EMPTY_TRACE}
            active={activeTrace}
            drawn={playback.drawn}
            places={places.data ?? []}
            entries={detail.data?.entries ?? []}
            focus={focus}
            head={playback.head}
          />
        </div>

        <Legend modes={overview.data.modes} selected={modes} onToggle={handleToggleMode} />

        <PlaybackControls
          playback={playback}
          label={openDate ? formatDayLong(openDate) : null}
        />
      </section>

      <Seismograph
        series={overview.data.dailyDistance}
        first={overview.data.firstDate}
        last={overview.data.lastDate}
        from={range.from}
        to={range.to}
        onChange={handleRange}
      />
    </main>
  )
}
