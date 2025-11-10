import { ScrollArea } from './ui/scroll-area';
import { Badge } from './ui/badge';
import { Message } from './Message';
import { formatDate } from '../utils/formatters';

export function SessionDetail({ session, loading, error }) {
  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-muted-foreground">Loading session...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-destructive">Failed to load session: {error}</p>
      </div>
    );
  }

  if (!session) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
        <h2 className="text-2xl font-semibold mb-2">Welcome to Retrochat</h2>
        <p>Select a session to view details</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="bg-muted/30 p-6 border-b space-y-3">
        <h2 className="text-2xl font-semibold">
          {session.project_name || 'Unnamed Project'}
        </h2>
        <div className="flex flex-wrap gap-4 text-sm text-muted-foreground">
          <div className="flex items-center gap-2">
            <span className="font-medium">Provider:</span>
            <Badge variant="secondary">{session.provider}</Badge>
          </div>
          <div>
            <span className="font-medium">Messages:</span> {session.messages.length}
          </div>
          <div>
            <span className="font-medium">Created:</span> {formatDate(session.created_at)}
          </div>
          <div>
            <span className="font-medium">Updated:</span> {formatDate(session.updated_at)}
          </div>
        </div>
      </div>

      <ScrollArea className="flex-1 p-6">
        <div className="space-y-4 max-w-4xl">
          {session.messages.map((message, index) => (
            <Message key={index} message={message} />
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}
