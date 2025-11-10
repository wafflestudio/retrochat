import { Badge } from './ui/badge';
import { Card } from './ui/card';
import { formatDate } from '../utils/formatters';
import { cn } from '@/lib/utils';

export function SessionItem({ session, isActive, onClick }) {
  return (
    <Card
      className={cn(
        'p-4 cursor-pointer hover:bg-accent transition-colors border-l-4',
        isActive
          ? 'bg-accent border-l-primary'
          : 'border-l-transparent hover:border-l-muted-foreground/20'
      )}
      onClick={onClick}
    >
      <div className="flex justify-between items-start mb-2">
        <Badge variant="secondary" className="text-xs uppercase">
          {session.provider}
        </Badge>
        <span className="text-xs text-muted-foreground">
          {formatDate(session.updated_at)}
        </span>
      </div>
      <div className="font-medium mb-1">
        {session.project_name || 'Unnamed Project'}
      </div>
      <div className="text-sm text-muted-foreground">
        {session.message_count} messages
      </div>
    </Card>
  );
}
