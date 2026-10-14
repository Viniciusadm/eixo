import { useQuery } from '@tanstack/react-query'

import { api, type DateRange } from '../api/client'

export function useOverview() {
  return useQuery({ queryKey: ['overview'], queryFn: api.overview })
}

export function useDays(range: DateRange | null) {
  return useQuery({
    queryKey: ['days', range?.from, range?.to],
    queryFn: () => api.days(range!),
    enabled: range !== null,
  })
}

export function useTrace(range: DateRange | null, modes: string[]) {
  return useQuery({
    queryKey: ['trace', range?.from, range?.to, modes.join(',')],
    queryFn: () => api.trace(range!, modes),
    enabled: range !== null,
  })
}

export function useDayDetail(date: string | null) {
  return useQuery({
    queryKey: ['day', date],
    queryFn: () => api.day(date!),
    enabled: date !== null,
  })
}

export function usePlaces(limit = 200) {
  return useQuery({ queryKey: ['places', limit], queryFn: () => api.places(limit) })
}
