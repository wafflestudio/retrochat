import { useState } from 'react';
import { Button } from './components/ui/button';
import { Input } from './components/ui/input';
import { Select } from './components/ui/select';
import { SessionList } from './components/SessionList';
import { SessionDetail } from './components/SessionDetail';
import { SearchModal } from './components/SearchModal';
import { useSessions } from './hooks/useSessions';
import { useSessionDetail } from './hooks/useSessionDetail';
import { useSearch } from './hooks/useSearch';
import { Search } from 'lucide-react';

function App() {
  const [selectedSessionId, setSelectedSessionId] = useState(null);
  const [provider, setProvider] = useState('');

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

  const handleSessionClick = (sessionId) => {
    setSelectedSessionId(sessionId);
  };

  const handleSearchResultClick = (sessionId) => {
    setSelectedSessionId(sessionId);
  };

  const handleSearch = () => {
    performSearch();
  };

  const handleKeyPress = (e) => {
    if (e.key === 'Enter') {
      handleSearch();
    }
  };

  return (
    <div className="flex flex-col h-screen">
      {/* Header */}
      <header className="bg-gradient-to-r from-violet-600 to-purple-600 text-white p-6 shadow-lg">
        <h1 className="text-3xl font-bold mb-1">Retrochat</h1>
        <p className="text-sm opacity-90">Browse and search your LLM chat history</p>
      </header>

      {/* Main Content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <aside className="w-96 bg-background border-r flex flex-col">
          {/* Search Bar */}
          <div className="p-4 border-b space-y-3">
            <div className="flex gap-2">
              <Input
                type="text"
                placeholder="Search messages..."
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyPress={handleKeyPress}
                className="flex-1"
              />
              <Button onClick={handleSearch} size="icon">
                <Search className="h-4 w-4" />
              </Button>
            </div>

            {/* Provider Filter */}
            <div>
              <label className="block text-sm font-medium mb-1">
                Filter by Provider
              </label>
              <Select
                value={provider}
                onChange={(e) => setProvider(e.target.value)}
              >
                <option value="">All Providers</option>
                <option value="Claude">Claude</option>
                <option value="Gemini">Gemini</option>
                <option value="Codex">Codex</option>
              </Select>
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
        <main className="flex-1 bg-background overflow-hidden">
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
