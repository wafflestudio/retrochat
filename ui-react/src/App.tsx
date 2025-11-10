import {
  Bot,
  ChevronLeft,
  ChevronRight,
  Clock,
  Loader2,
  MessageCircle,
  MessageSquare,
  Search,
  User,
} from 'lucide-react'
import { useState } from 'react'
import { Badge } from './components/ui/badge'
import { Button } from './components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './components/ui/card'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from './components/ui/dialog'
import { Input } from './components/ui/input'
import { ScrollArea } from './components/ui/scroll-area'
import { useSearch } from './hooks/useSearch'
import { useSessionDetail } from './hooks/useSessionDetail'
import { useSessions } from './hooks/useSessions'
import type { Message, Session } from './types'

function App() {
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null)
  const [provider] = useState<string>('')

  const {
    sessions,
    loading: sessionsLoading,
    error: sessionsError,
    currentPage,
    canGoPrev,
    canGoNext,
    nextPage,
    prevPage,
  } = useSessions(provider)

  const {
    session,
    loading: detailLoading,
    error: detailError,
  } = useSessionDetail(selectedSessionId)

  const {
    query,
    setQuery,
    results,
    loading: searchLoading,
    error: searchError,
    isOpen: searchOpen,
    performSearch,
    closeSearch,
  } = useSearch()

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr)
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
  }

  const formatTime = (dateStr: string) => {
    const date = new Date(dateStr)
    return date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' })
  }

  return (
    <div className="flex h-screen bg-background">
      {/* Sidebar */}
      <aside className="w-80 border-r flex flex-col">
        {/* Search Header */}
        <div className="p-4 border-b space-y-4">
          <div>
            <h1 className="text-2xl font-bold tracking-tight">Retrochat</h1>
            <p className="text-sm text-muted-foreground">LLM Chat History</p>
          </div>

          <div className="flex gap-2">
            <Input
              type="text"
              placeholder="Search messages..."
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && performSearch()}
              className="flex-1"
            />
            <Button onClick={() => performSearch()} size="icon" variant="secondary">
              <Search className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* Sessions List */}
        <ScrollArea className="flex-1">
          <div className="p-2 space-y-1">
            {sessionsLoading ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              </div>
            ) : sessionsError ? (
              <div className="p-4 text-center">
                <p className="text-sm text-destructive">{sessionsError}</p>
              </div>
            ) : sessions.length === 0 ? (
              <div className="p-8 text-center">
                <MessageSquare className="h-12 w-12 mx-auto text-muted-foreground mb-2" />
                <p className="text-sm text-muted-foreground">No sessions found</p>
              </div>
            ) : (
              sessions.map((session: Session) => (
                <Card
                  key={session.id}
                  className={`cursor-pointer transition-colors hover:bg-accent ${
                    selectedSessionId === session.id ? 'bg-accent border-primary' : ''
                  }`}
                  onClick={() => setSelectedSessionId(session.id)}
                >
                  <CardHeader className="p-4 pb-2">
                    <div className="flex items-start justify-between gap-2">
                      <CardTitle className="text-sm font-medium line-clamp-1">
                        {session.project_name || 'Untitled Session'}
                      </CardTitle>
                      <Badge variant="secondary" className="text-xs shrink-0">
                        {session.provider}
                      </Badge>
                    </div>
                    <CardDescription className="text-xs">
                      {formatDate(session.created_at)}
                    </CardDescription>
                  </CardHeader>
                  <CardContent className="p-4 pt-0">
                    <div className="flex items-center gap-1 text-xs text-muted-foreground">
                      <MessageCircle className="h-3 w-3" />
                      <span>{session.message_count} messages</span>
                    </div>
                  </CardContent>
                </Card>
              ))
            )}
          </div>
        </ScrollArea>

        {/* Pagination */}
        <div className="p-4 border-t">
          <div className="flex items-center justify-between gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={prevPage}
              disabled={!canGoPrev}
              className="flex-1"
            >
              <ChevronLeft className="h-4 w-4 mr-1" />
              Prev
            </Button>
            <span className="text-sm text-muted-foreground px-2">Page {currentPage}</span>
            <Button
              variant="outline"
              size="sm"
              onClick={nextPage}
              disabled={!canGoNext}
              className="flex-1"
            >
              Next
              <ChevronRight className="h-4 w-4 ml-1" />
            </Button>
          </div>
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 flex flex-col">
        {detailLoading ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground mx-auto mb-4" />
              <p className="text-sm text-muted-foreground">Loading session...</p>
            </div>
          </div>
        ) : detailError ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center">
              <p className="text-sm text-destructive">{detailError}</p>
            </div>
          </div>
        ) : !session ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center">
              <MessageSquare className="h-16 w-16 mx-auto text-muted-foreground mb-4" />
              <h2 className="text-xl font-semibold mb-2">No Session Selected</h2>
              <p className="text-sm text-muted-foreground">
                Select a session from the sidebar to view its messages
              </p>
            </div>
          </div>
        ) : (
          <>
            {/* Session Header */}
            <div className="border-b p-6">
              <div className="flex items-start justify-between">
                <div>
                  <div className="flex items-center gap-2 mb-2">
                    <h2 className="text-2xl font-bold">
                      {session.project_name || 'Untitled Session'}
                    </h2>
                    <Badge>{session.provider}</Badge>
                  </div>
                  <div className="flex items-center gap-4 text-sm text-muted-foreground">
                    <div className="flex items-center gap-1">
                      <Clock className="h-4 w-4" />
                      <span>{formatDate(session.created_at)}</span>
                    </div>
                    <div className="flex items-center gap-1">
                      <MessageCircle className="h-4 w-4" />
                      <span>{session.messages.length} messages</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Messages */}
            <ScrollArea className="flex-1 p-6">
              <div className="max-w-4xl mx-auto space-y-4">
                {session.messages.map((message: Message, idx: number) => (
                  <Card key={`${message.timestamp}-${idx}`}>
                    <CardHeader className="pb-3">
                      <div className="flex items-center gap-2">
                        {message.role.toLowerCase() === 'user' ? (
                          <User className="h-4 w-4" />
                        ) : (
                          <Bot className="h-4 w-4" />
                        )}
                        <CardTitle className="text-sm font-semibold capitalize">
                          {message.role}
                        </CardTitle>
                        <span className="text-xs text-muted-foreground ml-auto">
                          {formatTime(message.timestamp)}
                        </span>
                      </div>
                    </CardHeader>
                    <CardContent>
                      <p className="text-sm whitespace-pre-wrap leading-relaxed">
                        {message.content}
                      </p>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </ScrollArea>
          </>
        )}
      </main>

      {/* Search Dialog */}
      <Dialog open={searchOpen} onOpenChange={closeSearch}>
        <DialogContent className="max-w-3xl max-h-[80vh]">
          <DialogHeader>
            <DialogTitle>Search Results</DialogTitle>
          </DialogHeader>

          {searchLoading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : searchError ? (
            <div className="py-12 text-center">
              <p className="text-sm text-destructive">{searchError}</p>
            </div>
          ) : results.length === 0 ? (
            <div className="py-12 text-center">
              <Search className="h-12 w-12 mx-auto text-muted-foreground mb-2" />
              <p className="text-sm text-muted-foreground">No results found</p>
            </div>
          ) : (
            <ScrollArea className="max-h-[60vh]">
              <div className="space-y-2">
                {results.map((result, idx) => (
                  <Card
                    key={`${result.session_id}-${idx}`}
                    className="cursor-pointer hover:bg-accent transition-colors"
                    onClick={() => {
                      setSelectedSessionId(result.session_id)
                      closeSearch()
                    }}
                  >
                    <CardHeader className="pb-2">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          <Badge variant="secondary" className="text-xs">
                            {result.provider}
                          </Badge>
                          <Badge variant="outline" className="text-xs capitalize">
                            {result.role}
                          </Badge>
                        </div>
                        <span className="text-xs text-muted-foreground">
                          {formatDate(result.timestamp)}
                        </span>
                      </div>
                    </CardHeader>
                    <CardContent>
                      <p className="text-sm line-clamp-3">{result.content}</p>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </ScrollArea>
          )}
        </DialogContent>
      </Dialog>
    </div>
  )
}

export default App
