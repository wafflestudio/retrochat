'use client'

import { ChevronLeftIcon, ChevronRightIcon } from '@radix-ui/react-icons'
import { formatDistanceToNow } from 'date-fns'
import { Filter, Upload } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Kbd, KbdGroup } from '@/components/ui/kbd'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { getProviders, getSessions } from '@/lib/api'
import type { Session } from '@/types'

interface SessionListProps {
  provider: string | null
  onProviderChange: (provider: string | null) => void
  selectedSession: string | null
  onSessionSelect: (sessionId: string) => void
  onImport?: () => void
  refreshTrigger?: number
}

export function SessionList({
  provider,
  onProviderChange,
  selectedSession,
  onSessionSelect,
  onImport,
  refreshTrigger,
}: SessionListProps) {
  const [sessions, setSessions] = useState<Session[]>([])
  const [providers, setProviders] = useState<string[]>([])
  const [page, setPage] = useState(1)
  const [loading, setLoading] = useState(false)
  const pageSize = 20

  const loadProviders = useCallback(async () => {
    try {
      const data = await getProviders()
      setProviders(data)
    } catch (error) {
      console.error('Failed to load providers:', error)
    }
  }, [])

  const loadSessions = useCallback(async () => {
    setLoading(true)
    try {
      const data = await getSessions(page, pageSize, provider)
      setSessions(data)
    } catch (error) {
      console.error('Failed to load sessions:', error)
    } finally {
      setLoading(false)
    }
  }, [page, provider])

  useEffect(() => {
    loadProviders()
  }, [loadProviders])

  useEffect(() => {
    loadSessions()
  }, [loadSessions])

  // Refresh sessions when refreshTrigger changes
  useEffect(() => {
    if (refreshTrigger !== undefined) {
      loadSessions()
    }
  }, [refreshTrigger, loadSessions])

  return (
    <div className="h-full flex flex-col">
      <div className="p-4 border-b border-border flex-shrink-0">
        <Select
          value={provider || 'all'}
          onValueChange={(value) => {
            onProviderChange(value === 'all' ? null : value)
            setPage(1)
          }}
        >
          <SelectTrigger>
            <div className="flex items-center gap-2">
              <Filter className="w-4 h-4" />
              <SelectValue placeholder="All Providers" />
            </div>
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Providers</SelectItem>
            {providers.map((p) => (
              <SelectItem key={p} value={p}>
                {p}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="flex-1 overflow-y-auto min-h-0">
        <div className="p-2">
          {loading ? (
            <div className="text-center py-8 text-muted-foreground">Loading sessions...</div>
          ) : sessions.length === 0 ? (
            <div className="text-center py-8">
              <div className="text-muted-foreground mb-4">No sessions found</div>
              {onImport && (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button variant="outline" size="sm" onClick={onImport} className="gap-2">
                      <Upload className="w-4 h-4" />
                      Import Sessions
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    <div className="flex items-center gap-2">
                      <p>Import chat sessions from JSON or JSONL files</p>
                      <KbdGroup>
                        <Kbd>⌘</Kbd>
                        <Kbd>O</Kbd>
                      </KbdGroup>
                    </div>
                  </TooltipContent>
                </Tooltip>
              )}
            </div>
          ) : (
            <div className="space-y-2">
              {sessions.map((session) => (
                <button
                  type="button"
                  key={session.id}
                  onClick={() => onSessionSelect(session.id)}
                  className={`w-full text-left p-3 rounded-lg border transition-colors ${
                    selectedSession === session.id
                      ? 'bg-accent border-primary'
                      : 'bg-card border-border hover:bg-accent/50'
                  }`}
                >
                  <div className="flex items-start justify-between gap-2 mb-1">
                    <span className="text-sm font-medium text-foreground truncate">
                      {session.project_name || 'Untitled Session'}
                    </span>
                    <span className="text-xs text-muted-foreground shrink-0">
                      {formatDistanceToNow(new Date(session.created_at), {
                        addSuffix: true,
                      })}
                    </span>
                  </div>
                  <div className="flex items-center gap-2 text-xs text-muted-foreground">
                    <span className="px-2 py-0.5 rounded bg-primary/10 text-primary">
                      {session.provider}
                    </span>
                    <span>{session.message_count} messages</span>
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="p-4 border-t border-border flex items-center justify-between flex-shrink-0">
        <Button
          variant="outline"
          size="sm"
          onClick={() => setPage((p) => Math.max(1, p - 1))}
          disabled={page === 1 || loading}
        >
          <ChevronLeftIcon className="w-4 h-4" />
        </Button>
        <span className="text-sm text-muted-foreground">Page {page}</span>
        <Button
          variant="outline"
          size="sm"
          onClick={() => setPage((p) => p + 1)}
          disabled={sessions.length < pageSize || loading}
        >
          <ChevronRightIcon className="w-4 h-4" />
        </Button>
      </div>
    </div>
  )
}
