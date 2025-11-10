import { Card } from './ui/card';
import { Badge } from './ui/badge';
import { formatDateTime } from '../utils/formatters';
import { cn } from '@/lib/utils';

const roleColors = {
  user: 'bg-blue-50 border-l-blue-500',
  assistant: 'bg-sky-50 border-l-sky-500',
  system: 'bg-amber-50 border-l-amber-500',
};

export function Message({ message }) {
  const roleClass = roleColors[message.role.toLowerCase()] || 'bg-muted border-l-muted-foreground';

  return (
    <Card className={cn('p-4 border-l-4', roleClass)}>
      <div className="flex justify-between items-center mb-3">
        <Badge variant="outline" className="capitalize">
          {message.role}
        </Badge>
        <span className="text-xs text-muted-foreground">
          {formatDateTime(message.timestamp)}
        </span>
      </div>
      <div className="whitespace-pre-wrap break-words leading-relaxed">
        {message.content}
      </div>
    </Card>
  );
}
