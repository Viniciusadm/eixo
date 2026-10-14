import type { ModeTotal } from '../../api/types'
import { formatDistance } from '../../lib/format'
import { modeStyle } from '../../lib/modes'

interface Props {
  modes: ModeTotal[]
  selected: string[]
  onToggle: (mode: string) => void
}

export function Legend({ modes, selected, onToggle }: Props) {
  if (modes.length === 0) return null

  return (
    <div className="overlay legend">
      <p className="eyebrow">Modal</p>
      {modes.map((total) => {
        const style = modeStyle(total.mode)
        const on = selected.length === 0 || selected.includes(total.mode)
        return (
          <button
            key={total.mode}
            type="button"
            className={`legend__row${on ? ' legend__row--on' : ''}`}
            onClick={() => onToggle(total.mode)}
            aria-pressed={on}
          >
            <span className="legend__swatch" style={{ background: style.color }} />
            <span className="legend__label">{style.label}</span>
            <span className="legend__value">{formatDistance(total.distanceMeters)}</span>
          </button>
        )
      })}
    </div>
  )
}
