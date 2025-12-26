import {
  CalendarIcon,
  ChevronDownIcon,
  Cross2Icon,
  PersonIcon,
  ReloadIcon,
} from '@radix-ui/react-icons'
import { format, formatDistanceToNow } from 'date-fns'
import {
  Bot,
  Brain,
  Check,
  Copy,
  FileCode,
  MessageSquare,
  Terminal,
  TrendingUp,
} from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import { useHotkeys } from 'react-hotkeys-hook'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Kbd, KbdGroup } from '@/components/ui/kbd'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
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

  // Keyboard shortcut: Cmd+I to toggle analytics
  useHotkeys('meta+i', () => setShowAnalytics(!showAnalytics), { preventDefault: true })

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
    <TooltipProvider>
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
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setShowAnalytics(!showAnalytics)}
                  >
                    <TrendingUp className="w-4 h-4 mr-2" />
                    Analytics
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  <div className="flex items-center gap-1">
                    <span>Toggle Analytics</span>
                    <KbdGroup>
                      <Kbd>⌘</Kbd>
                      <Kbd>I</Kbd>
                    </KbdGroup>
                  </div>
                </TooltipContent>
              </Tooltip>
              <Button variant="ghost" size="icon" onClick={onClose}>
                <Cross2Icon className="w-4 h-4" />
              </Button>
            </div>
          </div>
        </div>

        {showAnalytics ? (
          <AnalyticsPanel sessionId={sessionId} />
        ) : (
          <div className="flex-1 overflow-y-auto overflow-x-hidden min-h-0">
            <div className="p-6 space-y-4 max-w-4xl mx-auto">
              {session.messages.map((message) => (
                <MessageRenderer key={message.id} message={message} />
              ))}
            </div>
          </div>
        )}
      </div>
    </TooltipProvider>
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

  if (messageType === 'slash_command') {
    return <SlashCommandMessage message={message} />
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
        </div>
      </div>
    </div>
  )
}

function SlashCommandMessage({ message }: { message: SessionWithMessages['messages'][0] }) {
  const [isOpen, setIsOpen] = useState(false)

  // Extract command name from content if available
  const commandName =
    message.content.match(/\[Slash Command: (.*?)\]/)?.[1] ||
    message.content.match(/<command-name>(.*?)<\/command-name>/)?.[1] ||
    'Command'

  return (
    <div className="flex gap-4 justify-start">
      <div className="flex gap-3 max-w-[80%]">
        <div className="w-8 h-8 rounded-full flex items-center justify-center shrink-0 bg-yellow-500/20 text-yellow-400 border border-yellow-500/30">
          <Terminal className="w-4 h-4" />
        </div>
        <div className="flex flex-col items-start flex-1">
          <Collapsible open={isOpen} onOpenChange={setIsOpen} className="w-full">
            <CollapsibleTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className="w-full justify-between p-3 h-auto bg-yellow-500/5 hover:bg-yellow-500/10 border border-yellow-500/20 rounded-lg"
              >
                <div className="flex items-center gap-2">
                  <Terminal className="w-4 h-4 text-yellow-400" />
                  <span className="text-sm font-medium text-yellow-300">{commandName}</span>
                  <Badge variant="outline" className="text-xs border-yellow-500/30 text-yellow-400">
                    Local
                  </Badge>
                </div>
                <ChevronDownIcon
                  className={`w-4 h-4 text-yellow-400 transition-transform ${
                    isOpen ? 'rotate-180' : ''
                  }`}
                />
              </Button>
            </CollapsibleTrigger>
            <CollapsibleContent className="mt-2">
              <div className="rounded-lg p-4 bg-card border border-border text-card-foreground">
                <p className="whitespace-pre-wrap leading-relaxed text-sm text-muted-foreground">
                  {message.content}
                </p>
              </div>
            </CollapsibleContent>
          </Collapsible>
        </div>
      </div>
    </div>
  )
}

function ToolRequestMessage({ message }: { message: SessionWithMessages['messages'][0] }) {
  const [isOpen, setIsOpen] = useState(false)

  return (
    <div className="flex gap-4 justify-start">
      <div className="flex gap-3 max-w-[80%]">
        <div className="w-8 h-8 rounded-full flex items-center justify-center shrink-0 bg-blue-500/20 text-blue-400 border border-blue-500/30">
          <ReloadIcon className="w-4 h-4" />
        </div>
        <div className="flex flex-col items-start flex-1">
          <Collapsible open={isOpen} onOpenChange={setIsOpen} className="w-full">
            <CollapsibleTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className="w-full justify-between p-3 h-auto bg-blue-500/5 hover:bg-blue-500/10 border border-blue-500/20 rounded-lg"
              >
                <div className="flex items-center gap-2">
                  <FileCode className="w-4 h-4 text-blue-400" />
                  <span className="text-sm font-medium text-blue-300">
                    {message.tool_operation?.tool_name || 'Tool Request'}
                  </span>
                  <Badge variant="outline" className="text-xs border-blue-500/30 text-blue-400">
                    Running
                  </Badge>
                </div>
                <ChevronDownIcon
                  className={`w-4 h-4 text-blue-400 transition-transform ${
                    isOpen ? 'rotate-180' : ''
                  }`}
                />
              </Button>
            </CollapsibleTrigger>
            <CollapsibleContent className="mt-2">
              <div className="rounded-lg p-4 bg-card border border-border text-card-foreground space-y-3">
                {message.content && (
                  <div>
                    <div className="text-xs font-medium text-muted-foreground mb-1">Content</div>
                    <p className="text-sm text-foreground leading-relaxed">{message.content}</p>
                  </div>
                )}
                {message.tool_operation?.raw_input && (
                  <div>
                    <div className="text-xs font-medium text-muted-foreground mb-1">Raw Input</div>
                    <pre className="text-xs text-muted-foreground bg-muted/50 p-3 rounded overflow-auto max-h-96 max-w-full whitespace-pre-wrap break-words">
                      {JSON.stringify(message.tool_operation.raw_input, null, 2)}
                    </pre>
                  </div>
                )}
                {message.tool_operation?.file_metadata && (
                  <div>
                    <div className="text-xs font-medium text-muted-foreground mb-1">File</div>
                    <div className="font-mono text-xs text-blue-400">
                      {message.tool_operation.file_metadata.file_path}
                    </div>
                  </div>
                )}
              </div>
            </CollapsibleContent>
          </Collapsible>
        </div>
      </div>
    </div>
  )
}

function ToolResultMessage({ message }: { message: SessionWithMessages['messages'][0] }) {
  const isSuccess = message.tool_operation?.success !== false
  const [isOpen, setIsOpen] = useState(false)

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
        <div className="flex flex-col items-start flex-1">
          <Collapsible open={isOpen} onOpenChange={setIsOpen} className="w-full">
            <CollapsibleTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className={`w-full justify-between p-3 h-auto border rounded-lg ${
                  isSuccess
                    ? 'bg-green-500/5 hover:bg-green-500/10 border-green-500/20'
                    : 'bg-red-500/5 hover:bg-red-500/10 border-red-500/20'
                }`}
              >
                <div className="flex items-center gap-2">
                  <FileCode
                    className={`w-4 h-4 ${isSuccess ? 'text-green-400' : 'text-red-400'}`}
                  />
                  <span
                    className={`text-sm font-medium ${isSuccess ? 'text-green-300' : 'text-red-300'}`}
                  >
                    {message.tool_operation?.tool_name || 'Tool Result'}
                  </span>
                  <Badge variant={isSuccess ? 'default' : 'destructive'} className="text-xs">
                    {isSuccess ? 'Success' : 'Failed'}
                  </Badge>
                </div>
                <ChevronDownIcon
                  className={`w-4 h-4 transition-transform ${
                    isSuccess ? 'text-green-400' : 'text-red-400'
                  } ${isOpen ? 'rotate-180' : ''}`}
                />
              </Button>
            </CollapsibleTrigger>
            <CollapsibleContent className="mt-2">
              <div className="rounded-lg p-4 bg-card border border-border text-card-foreground space-y-3">
                {message.content && (
                  <div>
                    <div className="text-xs font-medium text-muted-foreground mb-1">Content</div>
                    <p className="text-sm text-foreground leading-relaxed">{message.content}</p>
                  </div>
                )}
                {message.tool_operation?.result_summary && (
                  <div>
                    <div className="text-xs font-medium text-muted-foreground mb-1">
                      Result Summary
                    </div>
                    <p className="text-sm text-foreground leading-relaxed">
                      {message.tool_operation.result_summary}
                    </p>
                  </div>
                )}
                {message.tool_operation?.raw_result && (
                  <div>
                    <div className="text-xs font-medium text-muted-foreground mb-1">Raw Result</div>
                    <pre className="text-xs text-muted-foreground bg-muted/50 p-3 rounded overflow-auto max-h-96 max-w-full whitespace-pre-wrap break-words">
                      {JSON.stringify(message.tool_operation.raw_result, null, 2)}
                    </pre>
                  </div>
                )}
                {message.tool_operation && <ToolOperationCard operation={message.tool_operation} />}
              </div>
            </CollapsibleContent>
          </Collapsible>
        </div>
      </div>
    </div>
  )
}

function SimpleMessage({ message }: { message: SessionWithMessages['messages'][0] }) {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(message.content)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (error) {
      console.error('Failed to copy message:', error)
    }
  }

  return (
    <div className={`flex gap-4 ${message.role === 'User' ? 'justify-end' : 'justify-start'}`}>
      <div
        className={`flex gap-3 max-w-[80%] min-w-0 ${
          message.role === 'User' ? 'flex-row-reverse' : 'flex-row'
        }`}
      >
        <div
          className={`w-8 h-8 rounded-full flex items-center justify-center shrink-0 ${
            message.role === 'User'
              ? 'bg-primary text-primary-foreground'
              : 'bg-gray-200 text-gray-700 dark:bg-gray-700 dark:text-gray-200'
          }`}
        >
          {message.role === 'User' ? (
            <PersonIcon className="w-4 h-4" />
          ) : (
            <Bot className="w-4 h-4" />
          )}
        </div>
        <div
          className={`flex flex-col min-w-0 w-full ${
            message.role === 'User' ? 'items-end' : 'items-start'
          }`}
        >
          <div
            className={`rounded-lg p-4 prose prose-sm max-w-none dark:prose-invert break-words w-full prose-p:break-words prose-p:whitespace-normal prose-span:break-words ${
              message.role === 'User'
                ? 'bg-primary text-primary-foreground prose-p:text-primary-foreground prose-headings:text-primary-foreground prose-strong:text-primary-foreground prose-code:text-primary-foreground prose-a:text-primary-foreground prose-a:underline'
                : 'bg-gray-100 border border-gray-200 text-gray-900 dark:bg-gray-800 dark:border-gray-700 dark:text-gray-100 prose-p:text-gray-900 dark:prose-p:text-gray-100 prose-headings:text-gray-900 dark:prose-headings:text-gray-100 prose-strong:text-gray-900 dark:prose-strong:text-gray-100 prose-code:text-gray-900 dark:prose-code:text-gray-100 prose-pre:bg-gray-200 dark:prose-pre:bg-gray-900 prose-a:text-blue-600 dark:prose-a:text-blue-400'
            } prose-p:leading-relaxed`}
          >
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{message.content}</ReactMarkdown>
          </div>
          <div className="flex items-center gap-2 mt-1">
            <span className="text-xs text-muted-foreground">
              {formatDistanceToNow(new Date(message.timestamp), { addSuffix: true })}
            </span>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button variant="ghost" size="icon" className="h-6 w-6" onClick={handleCopy}>
                  {copied ? (
                    <Check className="w-3 h-3 text-green-500" />
                  ) : (
                    <Copy className="w-3 h-3" />
                  )}
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>{copied ? 'Copied!' : 'Copy message'}</p>
              </TooltipContent>
            </Tooltip>
          </div>
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
