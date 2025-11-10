import { Calendar, MessageSquare } from 'lucide-react'
import { cn } from '@/lib/utils'
import type { Session } from '@/types'
import { formatDate } from '../utils/formatters'
import { Badge } from './ui/badge'
import { Card } from './ui/card'

interface SessionItemProps {
  session: Session
  isActive: boolean
  onClick: () => void
}

export function SessionItem({ session, isActive, onClick }: SessionItemProps) {
  return (
    <Card
      className={cn(
        'group relative p-4 cursor-pointer transition-all duration-200 border-l-4 overflow-hidden',
        'hover:shadow-md hover:scale-[1.02]',
        isActive
          ? 'bg-primary/5 border-l-primary shadow-sm ring-2 ring-primary/20'
          : 'border-l-transparent hover:border-l-primary/50 hover:bg-accent/50'
      )}
      onClick={onClick}
    >
      {/* Subtle gradient overlay on hover */}
      <div
        className={cn(
          'absolute inset-0 bg-gradient-to-r from-primary/5 to-transparent opacity-0 transition-opacity duration-200',
          'group-hover:opacity-100',
          isActive && 'opacity-50'
        )}
      />

      <div className="relative z-10">
        <div className="flex justify-between items-start mb-3">
          <Badge
            variant="secondary"
            className={cn(
              'text-xs uppercase font-semibold tracking-wide',
              isActive && 'bg-primary/10 text-primary border-primary/20'
            )}
          >
            {session.provider}
          </Badge>
          <div className="flex items-center gap-1 text-xs text-muted-foreground">
            <Calendar className="h-3 w-3" />
            <span>{formatDate(session.updated_at)}</span>
          </div>
        </div>

        <div
          className={cn(
            'font-semibold mb-2 text-sm line-clamp-2 transition-colors',
            isActive ? 'text-primary' : 'text-foreground group-hover:text-primary'
          )}
        >
          {session.project_name || 'Unnamed Project'}
        </div>

        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <MessageSquare className="h-3.5 w-3.5" />
          <span className="font-medium">{session.message_count}</span>
          <span>messages</span>
        </div>
      </div>
    </Card>
  )
}
