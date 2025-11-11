import { History } from 'lucide-react'
import { useState } from 'react'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { SearchView } from './search-view'
import { SessionDetail } from './session-detail'
import { SessionList } from './session-list'

export function SessionManager() {
  const [activeTab, setActiveTab] = useState('sessions')
  const [selectedSession, setSelectedSession] = useState<string | null>(null)
  const [provider, setProvider] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState('')

  return (
    <div className="flex h-screen bg-background">
      {/* Sidebar */}
      <div className="w-80 border-r border-border bg-card flex flex-col h-full">
        <div className="p-4 border-b border-border flex-shrink-0">
          <div className="flex items-center gap-2 mb-4">
            <History className="w-6 h-6 text-primary" />
            <h1 className="text-xl font-semibold text-foreground">RetroChat</h1>
          </div>

          <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
            <TabsList className="grid w-full grid-cols-2">
              <TabsTrigger value="sessions">Sessions</TabsTrigger>
              <TabsTrigger value="search">Search</TabsTrigger>
            </TabsList>
          </Tabs>
        </div>

        <div className="flex-1 min-h-0">
          {activeTab === 'sessions' ? (
            <SessionList
              provider={provider}
              onProviderChange={setProvider}
              selectedSession={selectedSession}
              onSessionSelect={setSelectedSession}
            />
          ) : (
            <SearchView
              searchQuery={searchQuery}
              onSearchQueryChange={setSearchQuery}
              onSessionSelect={setSelectedSession}
            />
          )}
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex flex-col h-full min-w-0">
        {selectedSession ? (
          <SessionDetail sessionId={selectedSession} onClose={() => setSelectedSession(null)} />
        ) : (
          <div className="flex-1 flex items-center justify-center text-muted-foreground">
            <div className="text-center">
              <History className="w-16 h-16 mx-auto mb-4 opacity-20" />
              <p className="text-lg">Select a session to view details</p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
