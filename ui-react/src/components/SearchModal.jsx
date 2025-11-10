import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogClose } from './ui/dialog';
import { Card } from './ui/card';
import { Badge } from './ui/badge';
import { ScrollArea } from './ui/scroll-area';
import { formatDateTime } from '../utils/formatters';

function SearchResultItem({ result, onClick }) {
  return (
    <Card
      className="p-4 cursor-pointer hover:bg-accent transition-colors"
      onClick={onClick}
    >
      <div className="flex justify-between items-start mb-2">
        <Badge variant="outline" className="capitalize">
          {result.role}
        </Badge>
        <Badge variant="secondary" className="text-xs">
          {result.provider}
        </Badge>
      </div>
      <div className="text-sm mb-2 line-clamp-3 text-muted-foreground">
        {result.content}
      </div>
      <div className="text-xs text-muted-foreground">
        {formatDateTime(result.timestamp)}
      </div>
    </Card>
  );
}

export function SearchModal({ isOpen, onClose, results, loading, error, onResultClick }) {
  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogHeader>
        <DialogTitle>Search Results</DialogTitle>
        <DialogClose onClick={onClose} />
      </DialogHeader>

      <DialogContent className="p-6">
        {loading && (
          <div className="text-center text-muted-foreground py-8">
            Searching...
          </div>
        )}

        {error && (
          <div className="text-center text-destructive py-8">
            Search failed: {error}
          </div>
        )}

        {!loading && !error && results.length === 0 && (
          <div className="text-center text-muted-foreground py-8">
            No results found.
          </div>
        )}

        {!loading && !error && results.length > 0 && (
          <ScrollArea className="max-h-[60vh]">
            <div className="space-y-2">
              {results.map((result, index) => (
                <SearchResultItem
                  key={index}
                  result={result}
                  onClick={() => {
                    onResultClick(result.session_id);
                    onClose();
                  }}
                />
              ))}
            </div>
          </ScrollArea>
        )}
      </DialogContent>
    </Dialog>
  );
}
