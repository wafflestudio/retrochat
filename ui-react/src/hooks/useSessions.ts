import { useCallback, useEffect, useState } from 'react'
import type { Session } from '@/types'
import { getSessions } from '../utils/tauri'

interface UseSessionsReturn {
  sessions: Session[]
  loading: boolean
  error: string | null
  currentPage: number
  hasMore: boolean
  canGoPrev: boolean
  canGoNext: boolean
  nextPage: () => void
  prevPage: () => void
  refresh: () => void
}

/**
 * Hook for managing session data and pagination
 * @param provider - Filter by provider
 * @returns Sessions data and controls
 */
export function useSessions(provider: string = ''): UseSessionsReturn {
  const [sessions, setSessions] = useState<Session[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [currentPage, setCurrentPage] = useState(1)
  const [hasMore, setHasMore] = useState(true)
  const pageSize = 20

  const loadSessions = useCallback(async () => {
    try {
      setLoading(true)
      setError(null)
      const data = await getSessions(currentPage, pageSize, provider || null)
      setSessions(data)
      setHasMore(data.length >= pageSize)
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error'
      setError(errorMessage)
      console.error('Failed to load sessions:', err)
    } finally {
      setLoading(false)
    }
  }, [currentPage, provider])

  useEffect(() => {
    loadSessions()
  }, [loadSessions])

  const nextPage = () => {
    if (hasMore) {
      setCurrentPage((prev) => prev + 1)
    }
  }

  const prevPage = () => {
    if (currentPage > 1) {
      setCurrentPage((prev) => prev - 1)
    }
  }

  const refresh = () => {
    loadSessions()
  }

  return {
    sessions,
    loading,
    error,
    currentPage,
    hasMore,
    canGoPrev: currentPage > 1,
    canGoNext: hasMore,
    nextPage,
    prevPage,
    refresh,
  }
}
