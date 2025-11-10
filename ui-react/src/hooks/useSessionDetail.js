import { useState, useEffect } from 'react';
import { getSessionDetail } from '../utils/tauri';

/**
 * Hook for loading session detail
 * @param {string|null} sessionId - Session ID to load
 * @returns {Object} Session detail data and state
 */
export function useSessionDetail(sessionId) {
  const [session, setSession] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);

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
      setError(err.message);
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
