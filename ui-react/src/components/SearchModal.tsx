import { Dialog, DialogContent, DialogHeader, DialogTitle } from './ui/dialog';
import { Card } from './ui/card';
import { Badge } from './ui/badge';
import { ScrollArea } from './ui/scroll-area';
import { formatDateTime } from '../utils/formatters';
import { SearchResult } from '@/types';
import { Loader2, SearchX, User, Bot, Settings, MessageSquare } from 'lucide-react';
import { cn } from '@/lib/utils';

interface SearchResultItemProps {
  result: SearchResult;
  onClick: () => void;
}

function SearchResultItem({ result, onClick }: SearchResultItemProps) {
  const getRoleIcon = (role: string) => {
    const r = role.toLowerCase();
    if (r === 'user') return User;
    if (r === 'assistant') return Bot;
    if (r === 'system') return Settings;
    return MessageSquare;
  };

  const getRoleColor = (role: string) => {
    const r = role.toLowerCase();
    if (r === 'user') return 'bg-blue-500/10 text-blue-700 border-blue-500/30 dark:bg-blue-500/20 dark:text-blue-300';
    if (r === 'assistant') return 'bg-violet-500/10 text-violet-700 border-violet-500/30 dark:bg-violet-500/20 dark:text-violet-300';
    if (r === 'system') return 'bg-amber-500/10 text-amber-700 border-amber-500/30 dark:bg-amber-500/20 dark:text-amber-300';
    return 'bg-muted text-muted-foreground border-muted-foreground/30';
  };

  const RoleIcon = getRoleIcon(result.role);

  return (
    <Card
      className="group p-4 cursor-pointer transition-all duration-200 hover:shadow-md hover:scale-[1.01] hover:bg-accent/50 border-l-4 border-l-transparent hover:border-l-primary/50"
      onClick={onClick}
    >
      <div className="flex justify-between items-start mb-3 gap-3">
        <div className="flex items-center gap-2 flex-wrap">
          <Badge
            variant="outline"
            className={cn("capitalize font-semibold text-xs px-2 py-1 flex items-center gap-1", getRoleColor(result.role))}
          >
            <RoleIcon className="h-3 w-3" />
            {result.role}
          </Badge>
          <Badge
            variant="secondary"
            className="text-xs uppercase tracking-wide"
          >
            {result.provider}
          </Badge>
        </div>
        <span className="text-xs text-muted-foreground font-medium whitespace-nowrap">
          {formatDateTime(result.timestamp)}
        </span>
      </div>
      <div className="text-sm line-clamp-3 text-muted-foreground leading-relaxed group-hover:text-foreground transition-colors">
        {result.content}
      </div>
    </Card>
  );
}

interface SearchModalProps {
  isOpen: boolean;
  onClose: () => void;
  results: SearchResult[];
  loading: boolean;
  error: string | null;
  onResultClick: (sessionId: string) => void;
}

export function SearchModal({
  isOpen,
  onClose,
  results,
  loading,
  error,
  onResultClick,
}: SearchModalProps) {
  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle className="text-xl font-bold">Search Results</DialogTitle>
        </DialogHeader>

        {loading && (
          <div className="flex flex-col items-center justify-center py-16 gap-4">
            <Loader2 className="h-8 w-8 text-primary animate-spin" />
            <p className="text-sm text-muted-foreground font-medium">Searching messages...</p>
          </div>
        )}

        {error && (
          <div className="flex flex-col items-center justify-center py-16 gap-4">
            <div className="p-4 rounded-full bg-destructive/10">
              <SearchX className="h-8 w-8 text-destructive" />
            </div>
            <div className="text-center">
              <p className="text-sm font-semibold text-destructive mb-1">Search failed</p>
              <p className="text-xs text-muted-foreground">{error}</p>
            </div>
          </div>
        )}

        {!loading && !error && results.length === 0 && (
          <div className="flex flex-col items-center justify-center py-16 gap-4">
            <div className="p-4 rounded-full bg-muted">
              <SearchX className="h-8 w-8 text-muted-foreground" />
            </div>
            <div className="text-center">
              <p className="text-sm font-semibold text-foreground mb-1">No results found</p>
              <p className="text-xs text-muted-foreground">Try a different search term</p>
            </div>
          </div>
        )}

        {!loading && !error && results.length > 0 && (
          <div className="space-y-3">
            <div className="flex items-center justify-between pb-2">
              <p className="text-sm font-semibold text-muted-foreground">
                Found {results.length} {results.length === 1 ? 'result' : 'results'}
              </p>
            </div>
            <ScrollArea className="max-h-[60vh]">
              <div className="space-y-3 pr-4">
                {results.map((result, index) => (
                  <SearchResultItem
                    key={index}
                    result={result}
                    onClick={() => {
                      onResultClick(result.session_id);
                      onClose();
                    }}
                  />
                ))}
              </div>
            </ScrollArea>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
