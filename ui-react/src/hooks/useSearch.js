import { useState } from 'react';
import { searchMessages } from '../utils/tauri';

/**
 * Hook for search functionality
 * @returns {Object} Search state and controls
 */
export function useSearch() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [isOpen, setIsOpen] = useState(false);

  const performSearch = async (searchQuery = query) => {
    const trimmedQuery = searchQuery.trim();
    if (!trimmedQuery) return;

    try {
      setLoading(true);
      setError(null);
      setIsOpen(true);
      const data = await searchMessages(trimmedQuery, 50);
      setResults(data);
    } catch (err) {
      setError(err.message);
      console.error('Search failed:', err);
    } finally {
      setLoading(false);
    }
  };

  const closeSearch = () => {
    setIsOpen(false);
  };

  const clearSearch = () => {
    setQuery('');
    setResults([]);
    setError(null);
  };

  return {
    query,
    setQuery,
    results,
    loading,
    error,
    isOpen,
    performSearch,
    closeSearch,
    clearSearch,
  };
}
