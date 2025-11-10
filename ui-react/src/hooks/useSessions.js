import { useState, useEffect } from 'react';
import { getSessions } from '../utils/tauri';

/**
 * Hook for managing session data and pagination
 * @param {string} provider - Filter by provider
 * @returns {Object} Sessions data and controls
 */
export function useSessions(provider = '') {
  const [sessions, setSessions] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [currentPage, setCurrentPage] = useState(1);
  const [hasMore, setHasMore] = useState(true);
  const pageSize = 20;

  useEffect(() => {
    loadSessions();
  }, [currentPage, provider]);

  const loadSessions = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await getSessions(currentPage, pageSize, provider || null);
      setSessions(data);
      setHasMore(data.length >= pageSize);
    } catch (err) {
      setError(err.message);
      console.error('Failed to load sessions:', err);
    } finally {
      setLoading(false);
    }
  };

  const nextPage = () => {
    if (hasMore) {
      setCurrentPage((prev) => prev + 1);
    }
  };

  const prevPage = () => {
    if (currentPage > 1) {
      setCurrentPage((prev) => prev - 1);
    }
  };

  const refresh = () => {
    loadSessions();
  };

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
  };
}
