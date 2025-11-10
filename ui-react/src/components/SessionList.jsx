import { ScrollArea } from './ui/scroll-area';
import { Button } from './ui/button';
import { SessionItem } from './SessionItem';
import { ChevronLeft, ChevronRight } from 'lucide-react';

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
}) {
  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-muted-foreground">Loading sessions...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-destructive">Failed to load sessions: {error}</p>
      </div>
    );
  }

  if (sessions.length === 0) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-muted-foreground">No sessions found.</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="px-4 py-3 border-b">
        <h3 className="font-semibold">Sessions</h3>
      </div>

      <ScrollArea className="flex-1 px-4 py-2">
        <div className="space-y-2">
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

      <div className="border-t p-4 flex justify-between items-center">
        <Button
          variant="outline"
          size="sm"
          onClick={onPrevPage}
          disabled={!canGoPrev}
        >
          <ChevronLeft className="h-4 w-4 mr-1" />
          Previous
        </Button>
        <span className="text-sm text-muted-foreground">Page {currentPage}</span>
        <Button
          variant="outline"
          size="sm"
          onClick={onNextPage}
          disabled={!canGoNext}
        >
          Next
          <ChevronRight className="h-4 w-4 ml-1" />
        </Button>
      </div>
    </div>
  );
}
