import type {
  DayDetail,
  DaySummary,
  Overview,
  PlaceRanking,
  RangeStats,
  TraceCollection,
} from './types'

export type DateRange = {
  from: string
  to: string
}

async function request<T>(path: string, params?: Record<string, string | undefined>): Promise<T> {
  const query = new URLSearchParams()
  for (const [key, value] of Object.entries(params ?? {})) {
    if (value) query.set(key, value)
  }
  const suffix = query.toString()
  const response = await fetch(`/api/v1${path}${suffix ? `?${suffix}` : ''}`)

  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as { message?: string } | null
    throw new Error(body?.message ?? `A requisição para ${path} falhou (${response.status})`)
  }

  return response.json() as Promise<T>
}

export const api = {
  overview: () => request<Overview>('/timeline/overview'),
  days: (range: DateRange) => request<DaySummary[]>('/timeline/days', range),
  day: (date: string) => request<DayDetail>(`/timeline/days/${date}`),
  trace: (range: DateRange, modes?: string[]) =>
    request<TraceCollection>('/timeline/path', {
      ...range,
      modes: modes?.length ? modes.join(',') : undefined,
    }),
  places: (limit = 50) => request<PlaceRanking[]>('/timeline/places', { limit: String(limit) }),
  stats: (range: DateRange) => request<RangeStats>('/timeline/stats', range),
}
