import type { PlaceKind, TravelMode } from '../api/types'

interface ModeStyle {
  label: string
  color: string
  glyph: string
}

const MODES: Record<string, ModeStyle> = {
  WALKING: { label: 'A pé', color: '#5FD3C4', glyph: '↑' },
  IN_PASSENGER_VEHICLE: { label: 'Carro', color: '#F2A93B', glyph: '▭' },
  IN_SUBWAY: { label: 'Metrô', color: '#A98BFF', glyph: '≡' },
  IN_BUS: { label: 'Ônibus', color: '#4FA8F5', glyph: '▤' },
  CYCLING: { label: 'Bicicleta', color: '#8FD14F', glyph: '◎' },
  IN_TRAIN: { label: 'Trem', color: '#F2668B', glyph: '⊟' },
  IN_TAXI: { label: 'Táxi', color: '#FF7A45', glyph: '▯' },
  MOTORCYCLING: { label: 'Moto', color: '#FF9F6E', glyph: '◇' },
  RUNNING: { label: 'Corrida', color: '#7BE0A8', glyph: '↟' },
  FLYING: { label: 'Avião', color: '#C9D5DB', glyph: '△' },
}

export const UNTRACKED_COLOR = '#4A5C67'

export function modeStyle(mode: TravelMode | null | undefined): ModeStyle {
  if (!mode) return { label: 'Sem modal', color: UNTRACKED_COLOR, glyph: '·' }
  return MODES[mode] ?? { label: mode.toLowerCase(), color: UNTRACKED_COLOR, glyph: '·' }
}

export function modeColorExpression(): unknown[] {
  const pairs = Object.entries(MODES).flatMap(([mode, style]) => [mode, style.color])
  return ['match', ['get', 'mode'], ...pairs, UNTRACKED_COLOR]
}

export const PLACE_KIND: Record<PlaceKind, { label: string; short: string }> = {
  HOME: { label: 'Casa', short: 'CASA' },
  WORK: { label: 'Trabalho', short: 'TRAB' },
  UNKNOWN: { label: 'Lugar', short: '' },
}
