import { useState, useEffect } from 'react';
import { getSessionDetail } from '../utils/tauri';
import { SessionWithMessages } from '@/types';

interface UseSessionDetailReturn {
  session: SessionWithMessages | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * Hook for loading session detail
 * @param sessionId - Session ID to load
 * @returns Session detail data and state
 */
export function useSessionDetail(sessionId: string | null): UseSessionDetailReturn {
  const [session, setSession] = useState<SessionWithMessages | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!sessionId) {
      setSession(null);
      return;
    }

    loadDetail();
  }, [sessionId]);

  const loadDetail = async () => {
    if (!sessionId) return;

    try {
      setLoading(true);
      setError(null);
      const data = await getSessionDetail(sessionId);
      setSession(data);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error';
      setError(errorMessage);
      console.error('Failed to load session detail:', err);
    } finally {
      setLoading(false);
    }
  };

  return {
    session,
    loading,
    error,
    refresh: loadDetail,
  };
}
