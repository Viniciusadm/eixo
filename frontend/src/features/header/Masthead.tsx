import type { Overview } from '../../api/types'
import { formatCount, formatDayShort, formatDistance } from '../../lib/format'

interface Props {
  overview: Overview
  days: number
  distanceMeters: number
  places: number
}

export function Masthead({ overview, days, distanceMeters, places }: Props) {
  return (
    <header className="masthead">
      <div className="masthead__brand">
        <h1 className="masthead__wordmark">EIXO</h1>
        <p className="eyebrow">
          {overview.firstDate && overview.lastDate
            ? `${formatDayShort(overview.firstDate)} ${overview.firstDate.slice(0, 4)} — ${formatDayShort(
                overview.lastDate,
              )} ${overview.lastDate.slice(0, 4)}`
            : 'sem registros'}
        </p>
      </div>

      <div className="masthead__figures">
        <Figure label="Percorrido" value={formatDistance(distanceMeters)} />
        <Figure label="Dias" value={formatCount(days)} />
        <Figure label="Lugares" value={formatCount(places)} />
        <Figure label="Pontos GPS" value={formatCount(overview.tracePointCount)} muted />
      </div>
    </header>
  )
}

interface FigureProps {
  label: string
  value: string
  muted?: boolean
}

function Figure({ label, value, muted }: FigureProps) {
  return (
    <div className="figure">
      <span className={`figure__value${muted ? ' figure__value--muted' : ''}`}>{value}</span>
      <span className="eyebrow">{label}</span>
    </div>
  )
}
