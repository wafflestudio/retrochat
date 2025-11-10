import { useState } from 'react';
import { searchMessages } from '../utils/tauri';
import { SearchResult } from '@/types';

interface UseSearchReturn {
  query: string;
  setQuery: (query: string) => void;
  results: SearchResult[];
  loading: boolean;
  error: string | null;
  isOpen: boolean;
  performSearch: (searchQuery?: string) => Promise<void>;
  closeSearch: () => void;
  clearSearch: () => void;
}

/**
 * Hook for search functionality
 * @returns Search state and controls
 */
export function useSearch(): UseSearchReturn {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isOpen, setIsOpen] = useState(false);

  const performSearch = async (searchQuery: string = query) => {
    const trimmedQuery = searchQuery.trim();
    if (!trimmedQuery) return;

    try {
      setLoading(true);
      setError(null);
      setIsOpen(true);
      const data = await searchMessages(trimmedQuery, 50);
      setResults(data);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error';
      setError(errorMessage);
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
