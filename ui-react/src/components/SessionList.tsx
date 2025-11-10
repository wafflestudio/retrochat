import { AlertCircle, ChevronLeft, ChevronRight, Inbox, Loader2 } from 'lucide-react'
import type { Session } from '@/types'
import { SessionItem } from './SessionItem'
import { Button } from './ui/button'
import { ScrollArea } from './ui/scroll-area'

interface SessionListProps {
  sessions: Session[]
  loading: boolean
  error: string | null
  currentPage: number
  canGoPrev: boolean
  canGoNext: boolean
  onPrevPage: () => void
  onNextPage: () => void
  activeSessionId: string | null
  onSessionClick: (sessionId: string) => void
}

export function SessionList({
  sessions,
  loading,
  error,
  currentPage,
  canGoPrev,
  canGoNext,
  onPrevPage,
  onNextPage,
  activeSessionId,
  onSessionClick,
}: SessionListProps) {
  if (loading) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4 p-8">
        <Loader2 className="h-8 w-8 text-primary animate-spin" />
        <p className="text-sm text-muted-foreground font-medium">Loading sessions...</p>
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4 p-8">
        <div className="p-4 rounded-full bg-destructive/10">
          <AlertCircle className="h-8 w-8 text-destructive" />
        </div>
        <div className="text-center">
          <p className="text-sm font-semibold text-destructive mb-1">Failed to load sessions</p>
          <p className="text-xs text-muted-foreground">{error}</p>
        </div>
      </div>
    )
  }

  if (sessions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4 p-8">
        <div className="p-4 rounded-full bg-muted">
          <Inbox className="h-8 w-8 text-muted-foreground" />
        </div>
        <div className="text-center">
          <p className="text-sm font-semibold text-foreground mb-1">No sessions found</p>
          <p className="text-xs text-muted-foreground">Try adjusting your filters</p>
        </div>
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full">
      <div className="px-6 py-4 border-b border-border/50 bg-background/50">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
            Sessions
          </h3>
          <span className="text-xs font-medium text-muted-foreground bg-muted px-2 py-1 rounded">
            {sessions.length}
          </span>
        </div>
      </div>

      <ScrollArea className="flex-1 px-4 py-3">
        <div className="space-y-2.5">
          {sessions.map((session) => (
            <SessionItem
              key={session.id}
              session={session}
              isActive={session.id === activeSessionId}
              onClick={() => onSessionClick(session.id)}
            />
          ))}
        </div>
      </ScrollArea>

      <div className="border-t border-border/50 p-4 bg-background/50 backdrop-blur-sm">
        <div className="flex justify-between items-center gap-3">
          <Button
            variant="outline"
            size="sm"
            onClick={onPrevPage}
            disabled={!canGoPrev}
            className="flex-1 transition-all hover:bg-primary/5"
          >
            <ChevronLeft className="h-4 w-4 mr-1" />
            Prev
          </Button>
          <div className="px-3 py-1.5 rounded-md bg-muted">
            <span className="text-xs font-semibold text-foreground">Page {currentPage}</span>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={onNextPage}
            disabled={!canGoNext}
            className="flex-1 transition-all hover:bg-primary/5"
          >
            Next
            <ChevronRight className="h-4 w-4 ml-1" />
          </Button>
        </div>
      </div>
    </div>
  )
}
