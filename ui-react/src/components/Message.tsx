import { Card } from './ui/card';
import { Badge } from './ui/badge';
import { formatDateTime } from '../utils/formatters';
import { cn } from '@/lib/utils';
import { Message as MessageType } from '@/types';
import { User, Bot, Settings } from 'lucide-react';

interface MessageProps {
  message: MessageType;
}

const roleConfig = {
  user: {
    color: 'bg-blue-500/10 border-l-blue-500 dark:bg-blue-500/20',
    badgeColor: 'bg-blue-500/15 text-blue-700 border-blue-500/30 dark:bg-blue-500/20 dark:text-blue-300 dark:border-blue-500/40',
    icon: User,
  },
  assistant: {
    color: 'bg-violet-500/10 border-l-violet-500 dark:bg-violet-500/20',
    badgeColor: 'bg-violet-500/15 text-violet-700 border-violet-500/30 dark:bg-violet-500/20 dark:text-violet-300 dark:border-violet-500/40',
    icon: Bot,
  },
  system: {
    color: 'bg-amber-500/10 border-l-amber-500 dark:bg-amber-500/20',
    badgeColor: 'bg-amber-500/15 text-amber-700 border-amber-500/30 dark:bg-amber-500/20 dark:text-amber-300 dark:border-amber-500/40',
    icon: Settings,
  },
};

export function Message({ message }: MessageProps) {
  const role = message.role.toLowerCase();
  const config = roleConfig[role as keyof typeof roleConfig] || {
    color: 'bg-muted/50 border-l-muted-foreground',
    badgeColor: 'bg-muted text-muted-foreground border-muted-foreground/30',
    icon: User,
  };

  const Icon = config.icon;

  return (
    <Card className={cn(
      'p-5 border-l-4 transition-all duration-200 hover:shadow-md',
      config.color
    )}>
      <div className="flex justify-between items-center mb-4">
        <Badge
          variant="outline"
          className={cn(
            "capitalize font-semibold text-xs px-2.5 py-1 flex items-center gap-1.5",
            config.badgeColor
          )}
        >
          <Icon className="h-3 w-3" />
          {message.role}
        </Badge>
        <span className="text-xs text-muted-foreground font-medium">
          {formatDateTime(message.timestamp)}
        </span>
      </div>
      <div className="prose prose-sm dark:prose-invert max-w-none">
        <div className="whitespace-pre-wrap break-words leading-relaxed text-sm text-foreground/90">
          {message.content}
        </div>
      </div>
    </Card>
  );
}
