const TIME = new Intl.DateTimeFormat('pt-BR', { hour: '2-digit', minute: '2-digit' })
const DAY_LONG = new Intl.DateTimeFormat('pt-BR', {
  weekday: 'long',
  day: '2-digit',
  month: 'long',
  year: 'numeric',
})
const DAY_SHORT = new Intl.DateTimeFormat('pt-BR', { day: '2-digit', month: 'short' })
const MONTH = new Intl.DateTimeFormat('pt-BR', { month: 'short' })

export function isoDate(date: Date): string {
  const month = `${date.getMonth() + 1}`.padStart(2, '0')
  const day = `${date.getDate()}`.padStart(2, '0')
  return `${date.getFullYear()}-${month}-${day}`
}

export function parseIsoDate(value: string): Date {
  const [year, month, day] = value.split('-').map(Number)
  return new Date(year, month - 1, day)
}

export function addDays(value: string, amount: number): string {
  const date = parseIsoDate(value)
  date.setDate(date.getDate() + amount)
  return isoDate(date)
}

export function daysBetween(from: string, to: string): number {
  const start = parseIsoDate(from).getTime()
  const end = parseIsoDate(to).getTime()
  return Math.round((end - start) / 86_400_000)
}

export function clampDate(value: string, min: string, max: string): string {
  if (value < min) return min
  if (value > max) return max
  return value
}

export function formatTime(value: string): string {
  return TIME.format(new Date(value))
}

export function formatDayLong(value: string): string {
  return DAY_LONG.format(parseIsoDate(value))
}

export function formatDayShort(value: string): string {
  return DAY_SHORT.format(parseIsoDate(value)).replace('.', '')
}

export function formatMonth(value: string): string {
  return MONTH.format(parseIsoDate(value)).replace('.', '').toUpperCase()
}

export function formatDistance(meters: number): string {
  if (meters < 1000) return `${Math.round(meters)} m`
  const kilometres = meters / 1000
  return `${kilometres.toLocaleString('pt-BR', {
    minimumFractionDigits: kilometres < 100 ? 1 : 0,
    maximumFractionDigits: kilometres < 100 ? 1 : 0,
  })} km`
}

export function formatDuration(seconds: number): string {
  const total = Math.max(0, Math.round(seconds / 60))
  const hours = Math.floor(total / 60)
  const minutes = total % 60
  if (hours >= 24) {
    const days = Math.floor(hours / 24)
    return `${days} d ${hours % 24} h`
  }
  if (hours === 0) return `${minutes} min`
  if (minutes === 0) return `${hours} h`
  return `${hours} h ${minutes} min`
}

export function formatCount(value: number): string {
  return value.toLocaleString('pt-BR')
}

const RAIL_WEEKDAY = new Intl.DateTimeFormat('pt-BR', { weekday: 'short' })

export function formatDayRail(value: string): string {
  const date = parseIsoDate(value)
  const weekday = RAIL_WEEKDAY.format(date).replace('.', '').slice(0, 3)
  return `${weekday} ${formatDayShort(value).replace(' de ', ' ')}`
}

export function formatSpan(startedAt: string, endedAt: string): string {
  const start = new Date(startedAt)
  const end = new Date(endedAt)
  const overnight = Math.round(
    (new Date(end).setHours(0, 0, 0, 0) - new Date(start).setHours(0, 0, 0, 0)) / 86_400_000,
  )
  const suffix = overnight > 0 ? `+${overnight}` : ''
  return `${TIME.format(start)}\u2013${TIME.format(end)}${suffix}`
}
