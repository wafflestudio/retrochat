import { ScrollArea } from './ui/scroll-area';
import { Badge } from './ui/badge';
import { Message } from './Message';
import { formatDate } from '../utils/formatters';
import { SessionWithMessages } from '@/types';
import { Loader2, AlertCircle, MessageSquareText, Calendar, Clock, Layers } from 'lucide-react';

interface SessionDetailProps {
  session: SessionWithMessages | null;
  loading: boolean;
  error: string | null;
}

export function SessionDetail({ session, loading, error }: SessionDetailProps) {
  if (loading) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4">
        <Loader2 className="h-10 w-10 text-primary animate-spin" />
        <p className="text-sm text-muted-foreground font-medium">Loading session details...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4 p-8">
        <div className="p-4 rounded-full bg-destructive/10">
          <AlertCircle className="h-10 w-10 text-destructive" />
        </div>
        <div className="text-center">
          <p className="text-sm font-semibold text-destructive mb-1">Failed to load session</p>
          <p className="text-xs text-muted-foreground">{error}</p>
        </div>
      </div>
    );
  }

  if (!session) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-center p-8 gap-6">
        <div className="p-6 rounded-full bg-gradient-to-br from-primary/20 to-primary/5">
          <MessageSquareText className="h-16 w-16 text-primary" />
        </div>
        <div className="space-y-2">
          <h2 className="text-2xl font-bold text-foreground">Welcome to Retrochat</h2>
          <p className="text-sm text-muted-foreground max-w-md">
            Select a session from the sidebar to view your conversation history
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Session Header */}
      <div className="relative bg-gradient-to-br from-muted/50 to-muted/30 border-b border-border/50 backdrop-blur-sm">
        <div className="absolute inset-0 bg-gradient-to-r from-primary/5 to-transparent opacity-50" />
        <div className="relative px-8 py-6 space-y-4">
          <div className="flex items-start justify-between">
            <div className="space-y-2 flex-1">
              <div className="flex items-center gap-3">
                <Badge
                  variant="secondary"
                  className="uppercase font-semibold tracking-wide px-3 py-1"
                >
                  {session.provider}
                </Badge>
              </div>
              <h2 className="text-3xl font-bold text-foreground tracking-tight">
                {session.project_name || 'Unnamed Project'}
              </h2>
            </div>
          </div>

          {/* Metadata Grid */}
          <div className="grid grid-cols-3 gap-4 pt-2">
            <div className="flex items-center gap-2 text-sm">
              <div className="p-2 rounded-lg bg-background/50">
                <Layers className="h-4 w-4 text-primary" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground font-medium">Messages</p>
                <p className="font-semibold text-foreground">{session.messages.length}</p>
              </div>
            </div>
            <div className="flex items-center gap-2 text-sm">
              <div className="p-2 rounded-lg bg-background/50">
                <Calendar className="h-4 w-4 text-primary" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground font-medium">Created</p>
                <p className="font-semibold text-foreground">{formatDate(session.created_at)}</p>
              </div>
            </div>
            <div className="flex items-center gap-2 text-sm">
              <div className="p-2 rounded-lg bg-background/50">
                <Clock className="h-4 w-4 text-primary" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground font-medium">Updated</p>
                <p className="font-semibold text-foreground">{formatDate(session.updated_at)}</p>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Messages */}
      <ScrollArea className="flex-1 p-8 bg-muted/20">
        <div className="space-y-4 max-w-5xl mx-auto">
          {session.messages.map((message, index) => (
            <Message key={index} message={message} />
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}
