import {
  CalendarIcon,
  ChevronDownIcon,
  Cross2Icon,
  PersonIcon,
  ReloadIcon,
} from '@radix-ui/react-icons'
import { format } from 'date-fns'
import { Bot, Brain, FileCode, MessageSquare, TrendingUp } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { getSessionDetail } from '@/lib/api'
import type { SessionWithMessages, ToolOperation } from '@/types'
import { AnalyticsPanel } from './analytics-panel'

interface SessionDetailProps {
  sessionId: string
  onClose: () => void
}

export function SessionDetail({ sessionId, onClose }: SessionDetailProps) {
  const [session, setSession] = useState<SessionWithMessages | null>(null)
  const [loading, setLoading] = useState(false)
  const [showAnalytics, setShowAnalytics] = useState(false)

  const loadSessionDetail = useCallback(async () => {
    setLoading(true)
    try {
      const data = await getSessionDetail(sessionId)
      setSession(data)
    } catch (error) {
      console.error('Failed to load session detail:', error)
    } finally {
      setLoading(false)
    }
  }, [sessionId])

  useEffect(() => {
    loadSessionDetail()
  }, [loadSessionDetail])

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <p className="text-muted-foreground">Loading session...</p>
      </div>
    )
  }

  if (!session) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <p className="text-muted-foreground">Session not found</p>
      </div>
    )
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="border-b border-border bg-card p-4 flex-shrink-0">
        <div className="flex items-start justify-between">
          <div>
            <h2 className="text-xl font-semibold text-foreground mb-2">
              {session.project_name || 'Untitled Session'}
            </h2>
            <div className="flex items-center gap-4 text-sm text-muted-foreground">
              <span className="flex items-center gap-1">
                <Badge variant="secondary">{session.provider}</Badge>
              </span>
              <span className="flex items-center gap-1">
                <MessageSquare className="w-4 h-4" />
                {session.message_count} messages
              </span>
              <span className="flex items-center gap-1">
                <CalendarIcon className="w-4 h-4" />
                {format(new Date(session.created_at), 'MMM d, yyyy HH:mm')}
              </span>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={() => setShowAnalytics(!showAnalytics)}>
              <TrendingUp className="w-4 h-4 mr-2" />
              Analytics
            </Button>
            <Button variant="ghost" size="icon" onClick={onClose}>
              <Cross2Icon className="w-4 h-4" />
            </Button>
          </div>
        </div>
      </div>

      {showAnalytics ? (
        <AnalyticsPanel sessionId={sessionId} />
      ) : (
        <div className="flex-1 overflow-y-auto min-h-0">
          <div className="p-6 space-y-4 max-w-4xl mx-auto">
            {session.messages.map((message) => (
              <MessageRenderer key={message.id} message={message} />
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

function MessageRenderer({ message }: { message: SessionWithMessages['messages'][0] }) {
  const messageType = message.message_type

  if (messageType === 'thinking') {
    return <ThinkingMessage message={message} />
  }

  if (messageType === 'tool_request') {
    return <ToolRequestMessage message={message} />
  }

  if (messageType === 'tool_result') {
    return <ToolResultMessage message={message} />
  }

  return <SimpleMessage message={message} />
}

function ThinkingMessage({ message }: { message: SessionWithMessages['messages'][0] }) {
  const [isOpen, setIsOpen] = useState(false)

  return (
    <div className="flex gap-4 justify-start">
      <div className="flex gap-3 max-w-[80%]">
        <div className="w-8 h-8 rounded-full flex items-center justify-center shrink-0 bg-purple-500/20 text-purple-400 border border-purple-500/30">
          <Brain className="w-4 h-4" />
        </div>
        <div className="flex flex-col items-start flex-1">
          <Collapsible open={isOpen} onOpenChange={setIsOpen} className="w-full">
            <CollapsibleTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className="w-full justify-between p-3 h-auto bg-purple-500/5 hover:bg-purple-500/10 border border-purple-500/20 rounded-lg"
              >
                <div className="flex items-center gap-2">
                  <Brain className="w-4 h-4 text-purple-400" />
                  <span className="text-sm font-medium text-purple-300">Thinking...</span>
                </div>
                <ChevronDownIcon
                  className={`w-4 h-4 text-purple-400 transition-transform ${
                    isOpen ? 'rotate-180' : ''
                  }`}
                />
              </Button>
            </CollapsibleTrigger>
            <CollapsibleContent className="mt-2">
              <div className="rounded-lg p-4 bg-card border border-border text-card-foreground">
                <p className="whitespace-pre-wrap leading-relaxed text-sm text-muted-foreground italic">
                  {message.content}
                </p>
              </div>
            </CollapsibleContent>
          </Collapsible>
          <span className="text-xs text-muted-foreground mt-1">
            {format(new Date(message.timestamp), 'HH:mm:ss')}
          </span>
        </div>
      </div>
    </div>
  )
}

function ToolRequestMessage({ message }: { message: SessionWithMessages['messages'][0] }) {
  return (
    <div className="flex gap-4 justify-start">
      <div className="flex gap-3 max-w-[80%]">
        <div className="w-8 h-8 rounded-full flex items-center justify-center shrink-0 bg-blue-500/20 text-blue-400 border border-blue-500/30">
          <ReloadIcon className="w-4 h-4 animate-spin" />
        </div>
        <div className="flex flex-col items-start">
          <div className="rounded-lg p-4 bg-card border border-blue-500/30 text-card-foreground">
            <div className="flex items-center gap-2 mb-2">
              <FileCode className="w-4 h-4 text-blue-400" />
              <span className="text-sm font-medium text-blue-300">
                {message.tool_operation?.tool_name || 'Tool Request'}
              </span>
              <Badge variant="outline" className="text-xs border-blue-500/30 text-blue-400">
                Running
              </Badge>
            </div>
            <p className="text-sm text-muted-foreground">{message.content}</p>
            {message.tool_operation?.file_metadata && (
              <div className="mt-2 p-2 rounded bg-muted/50 text-xs">
                <div className="font-mono text-blue-400">
                  {message.tool_operation.file_metadata.file_path}
                </div>
              </div>
            )}
          </div>
          <span className="text-xs text-muted-foreground mt-1">
            {format(new Date(message.timestamp), 'HH:mm:ss')}
          </span>
        </div>
      </div>
    </div>
  )
}

function ToolResultMessage({ message }: { message: SessionWithMessages['messages'][0] }) {
  const isSuccess = message.tool_operation?.success !== false

  return (
    <div className="flex gap-4 justify-start">
      <div className="flex gap-3 max-w-[80%]">
        <div
          className={`w-8 h-8 rounded-full flex items-center justify-center shrink-0 ${
            isSuccess
              ? 'bg-green-500/20 text-green-400 border border-green-500/30'
              : 'bg-red-500/20 text-red-400 border border-red-500/30'
          }`}
        >
          <FileCode className="w-4 h-4" />
        </div>
        <div className="flex flex-col items-start">
          <div
            className={`rounded-lg p-4 border text-card-foreground ${
              isSuccess ? 'bg-card border-green-500/30' : 'bg-card border-red-500/30'
            }`}
          >
            <div className="flex items-center gap-2 mb-2">
              <FileCode className={`w-4 h-4 ${isSuccess ? 'text-green-400' : 'text-red-400'}`} />
              <span
                className={`text-sm font-medium ${isSuccess ? 'text-green-300' : 'text-red-300'}`}
              >
                {message.tool_operation?.tool_name || 'Tool Result'}
              </span>
              <Badge variant={isSuccess ? 'default' : 'destructive'} className="text-xs">
                {isSuccess ? 'Success' : 'Failed'}
              </Badge>
            </div>
            {message.content && (
              <p className="text-sm text-muted-foreground mb-2">{message.content}</p>
            )}
            {message.tool_operation && <ToolOperationCard operation={message.tool_operation} />}
          </div>
          <span className="text-xs text-muted-foreground mt-1">
            {format(new Date(message.timestamp), 'HH:mm:ss')}
          </span>
        </div>
      </div>
    </div>
  )
}

function SimpleMessage({ message }: { message: SessionWithMessages['messages'][0] }) {
  console.log(message)
  return (
    <div className={`flex gap-4 ${message.role === 'User' ? 'justify-end' : 'justify-start'}`}>
      <div
        className={`flex gap-3 max-w-[80%] ${
          message.role === 'User' ? 'flex-row-reverse' : 'flex-row'
        }`}
      >
        <div
          className={`w-8 h-8 rounded-full flex items-center justify-center shrink-0 ${
            message.role === 'User'
              ? 'bg-primary text-primary-foreground'
              : 'bg-secondary text-secondary-foreground'
          }`}
        >
          {message.role === 'User' ? (
            <PersonIcon className="w-4 h-4" />
          ) : (
            <Bot className="w-4 h-4" />
          )}
        </div>
        <div className={`flex flex-col ${message.role === 'User' ? 'items-end' : 'items-start'}`}>
          <div
            className={`rounded-lg p-4 ${
              message.role === 'User'
                ? 'bg-primary text-primary-foreground'
                : 'bg-card border border-border text-card-foreground'
            }`}
          >
            <p className="whitespace-pre-wrap leading-relaxed">{message.content}</p>
          </div>
          <span className="text-xs text-muted-foreground mt-1">
            {format(new Date(message.timestamp), 'HH:mm:ss')}
          </span>
        </div>
      </div>
    </div>
  )
}

function ToolOperationCard({ operation }: { operation: ToolOperation }) {
  return (
    <div className="space-y-1">
      {operation.result_summary && (
        <p className="text-sm text-muted-foreground">{operation.result_summary}</p>
      )}

      {operation.file_metadata && (
        <div className="mt-2 p-2 rounded bg-muted/50 text-xs space-y-1">
          <div className="font-mono text-primary">{operation.file_metadata.file_path}</div>
          <div className="flex items-center gap-3 text-muted-foreground">
            {operation.file_metadata.lines_added !== null && (
              <span className="text-green-500">+{operation.file_metadata.lines_added}</span>
            )}
            {operation.file_metadata.lines_removed !== null && (
              <span className="text-red-500">-{operation.file_metadata.lines_removed}</span>
            )}
            {operation.file_metadata.file_extension && (
              <span>{operation.file_metadata.file_extension}</span>
            )}
          </div>
        </div>
      )}
    </div>
  )
}
