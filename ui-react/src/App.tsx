import { useState } from "react";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "./components/ui/select";
import { SessionList } from "./components/SessionList";
import { SessionDetail } from "./components/SessionDetail";
import { SearchModal } from "./components/SearchModal";
import { useSessions } from "./hooks/useSessions";
import { useSessionDetail } from "./hooks/useSessionDetail";
import { useSearch } from "./hooks/useSearch";
import { Search } from "lucide-react";

function App() {
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(
    null
  );
  const [provider, setProvider] = useState<string>("");

  // Custom hooks
  const {
    sessions,
    loading: sessionsLoading,
    error: sessionsError,
    currentPage,
    canGoPrev,
    canGoNext,
    nextPage,
    prevPage,
  } = useSessions(provider);

  const {
    session,
    loading: detailLoading,
    error: detailError,
  } = useSessionDetail(selectedSessionId);

  const {
    query,
    setQuery,
    results,
    loading: searchLoading,
    error: searchError,
    isOpen: searchOpen,
    performSearch,
    closeSearch,
  } = useSearch();

  const handleSessionClick = (sessionId: string) => {
    setSelectedSessionId(sessionId);
  };

  const handleSearchResultClick = (sessionId: string) => {
    setSelectedSessionId(sessionId);
  };

  const handleSearch = () => {
    performSearch();
  };

  const handleKeyPress = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      handleSearch();
    }
  };

  return (
    <div className="flex flex-col h-screen bg-background">
      {/* Header */}
      <header className="relative bg-gradient-to-r from-violet-600 via-purple-600 to-fuchsia-600 text-white shadow-xl">
        <div className="absolute inset-0 bg-gradient-to-b from-black/10 to-transparent" />
        <div className="relative px-8 py-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-4xl font-bold tracking-tight mb-2 bg-clip-text text-transparent bg-gradient-to-r from-white to-white/90">
                Retrochat
              </h1>
              <p className="text-sm text-white/80 font-medium">
                Browse and search your LLM chat history
              </p>
            </div>
            <div className="text-right">
              <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-white/10 backdrop-blur-sm border border-white/20">
                <div className="h-2 w-2 rounded-full bg-green-400 animate-pulse" />
                <span className="text-sm font-medium">Connected</span>
              </div>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <aside className="w-[420px] bg-muted/30 border-r border-border/50 flex flex-col shadow-sm">
          {/* Search Bar */}
          <div className="p-6 border-b border-border/50 space-y-4 bg-background/50 backdrop-blur-sm">
            <div>
              <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2 block">
                Search
              </label>
              <div className="flex gap-2">
                <Input
                  type="text"
                  placeholder="Search messages..."
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  onKeyPress={handleKeyPress}
                  className="flex-1 h-10 bg-background/80 border-border/50 focus:border-primary transition-colors"
                />
                <Button
                  onClick={handleSearch}
                  size="icon"
                  className="h-10 w-10 bg-primary hover:bg-primary/90 shadow-sm"
                >
                  <Search className="h-4 w-4" />
                </Button>
              </div>
            </div>

            {/* Provider Filter */}
            <div>
              <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2 block">
                Provider
              </label>
              {/* <Select value={provider} onValueChange={setProvider}>
                <SelectTrigger className="bg-background/80 border-border/50 focus:border-primary transition-colors h-10">
                  <SelectValue placeholder="All Providers" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="">All Providers</SelectItem>
                  <SelectItem value="Claude">Claude</SelectItem>
                  <SelectItem value="Gemini">Gemini</SelectItem>
                  <SelectItem value="Codex">Codex</SelectItem>
                </SelectContent>
              </Select> */}
            </div>
          </div>

          {/* Session List */}
          <SessionList
            sessions={sessions}
            loading={sessionsLoading}
            error={sessionsError}
            currentPage={currentPage}
            canGoPrev={canGoPrev}
            canGoNext={canGoNext}
            onPrevPage={prevPage}
            onNextPage={nextPage}
            activeSessionId={selectedSessionId}
            onSessionClick={handleSessionClick}
          />
        </aside>

        {/* Main Detail View */}
        <main className="flex-1 overflow-hidden">
          <SessionDetail
            session={session}
            loading={detailLoading}
            error={detailError}
          />
        </main>
      </div>

      {/* Search Modal */}
      <SearchModal
        isOpen={searchOpen}
        onClose={closeSearch}
        results={results}
        loading={searchLoading}
        error={searchError}
        onResultClick={handleSearchResultClick}
      />
    </div>
  );
}

export default App;
