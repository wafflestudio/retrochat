import { useCallback, useEffect, useState } from 'react'
import type { SessionWithMessages } from '@/types'
import { getSessionDetail } from '../utils/tauri'

interface UseSessionDetailReturn {
  session: SessionWithMessages | null
  loading: boolean
  error: string | null
  refresh: () => void
}

/**
 * Hook for loading session detail
 * @param sessionId - Session ID to load
 * @returns Session detail data and state
 */
export function useSessionDetail(sessionId: string | null): UseSessionDetailReturn {
  const [session, setSession] = useState<SessionWithMessages | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const loadDetail = useCallback(async () => {
    if (!sessionId) return

    try {
      setLoading(true)
      setError(null)
      const data = await getSessionDetail(sessionId)
      setSession(data)
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error'
      setError(errorMessage)
      console.error('Failed to load session detail:', err)
    } finally {
      setLoading(false)
    }
  }, [sessionId])

  useEffect(() => {
    if (!sessionId) {
      setSession(null)
      return
    }

    loadDetail()
  }, [sessionId, loadDetail])

  return {
    session,
    loading,
    error,
    refresh: loadDetail,
  }
}
