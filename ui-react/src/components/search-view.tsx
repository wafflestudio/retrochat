'use client'

import { MagnifyingGlassIcon } from '@radix-ui/react-icons'
import { formatDistanceToNow } from 'date-fns'
import { useCallback, useEffect, useState } from 'react'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { searchMessages } from '@/lib/api'
import type { SearchResult } from '@/types'

interface SearchViewProps {
  searchQuery: string
  onSearchQueryChange: (query: string) => void
  onSessionSelect: (sessionId: string) => void
}

export function SearchView({ searchQuery, onSearchQueryChange, onSessionSelect }: SearchViewProps) {
  const [results, setResults] = useState<SearchResult[]>([])
  const [loading, setLoading] = useState(false)

  const performSearch = useCallback(async () => {
    setLoading(true)
    try {
      const data = await searchMessages(searchQuery, 50)
      setResults(data)
    } catch (error) {
      console.error('[v0] Failed to search messages:', error)
    } finally {
      setLoading(false)
    }
  }, [searchQuery])

  useEffect(() => {
    if (searchQuery.trim().length >= 3) {
      performSearch()
    } else {
      setResults([])
    }
  }, [searchQuery, performSearch])

  return (
    <div className="flex-1 flex flex-col h-full">
      <div className="p-4 border-b border-border flex-shrink-0">
        <div className="relative">
          <MagnifyingGlassIcon className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <Input
            type="text"
            placeholder="Search messages..."
            value={searchQuery}
            onChange={(e) => onSearchQueryChange(e.target.value)}
            className="pl-9"
          />
        </div>
      </div>

      <ScrollArea className="flex-1 overflow-hidden">
        <div className="p-2 h-full">
          {searchQuery.trim().length < 3 ? (
            <div className="text-center py-8 text-muted-foreground">
              Enter at least 3 characters to search
            </div>
          ) : loading ? (
            <div className="text-center py-8 text-muted-foreground">Searching...</div>
          ) : results.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">No results found</div>
          ) : (
            <div className="space-y-2">
              {results.map((result, index) => (
                <button
                  type="button"
                  key={`${result.session_id}-${index}`}
                  onClick={() => onSessionSelect(result.session_id)}
                  className="w-full text-left p-3 rounded-lg border border-border bg-card hover:bg-accent/50 transition-colors"
                >
                  <div className="flex items-start justify-between gap-2 mb-2">
                    <span className="text-xs px-2 py-0.5 rounded bg-primary/10 text-primary">
                      {result.provider}
                    </span>
                    <span className="text-xs text-muted-foreground">{result.role}</span>
                  </div>
                  <p className="text-sm text-foreground line-clamp-3 leading-relaxed mb-1">
                    {result.content}
                  </p>
                  <span className="text-xs text-muted-foreground">
                    {formatDistanceToNow(new Date(result.timestamp), {
                      addSuffix: true,
                    })}
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>
      </ScrollArea>
    </div>
  )
}
