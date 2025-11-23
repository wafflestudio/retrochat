import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { stat } from '@tauri-apps/plugin-fs'
import { FileText, History, Upload } from 'lucide-react'
import { useTheme } from 'next-themes'
import { useCallback, useEffect, useState } from 'react'
import { useHotkeys } from 'react-hotkeys-hook'
import { open } from '@tauri-apps/plugin-dialog'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Kbd, KbdGroup } from '@/components/ui/kbd'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { SearchView } from './search-view'
import { SessionDetail } from './session-detail'
import { SessionList } from './session-list'
import { ThemeToggle } from './theme-toggle'

interface FileInfo {
  path: string
  name: string
  size: number
}

export function SessionManager() {
  const [activeTab, setActiveTab] = useState('sessions')
  const [selectedSession, setSelectedSession] = useState<string | null>(null)
  const [provider, setProvider] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState('')
  const { theme, setTheme } = useTheme()
  const [importDialogOpen, setImportDialogOpen] = useState(false)
  const [filesToImport, setFilesToImport] = useState<FileInfo[]>([])
  const [refreshTrigger, setRefreshTrigger] = useState(0)

  // Keyboard shortcuts
  useHotkeys('meta+1', () => setActiveTab('sessions'), { preventDefault: true })
  useHotkeys('meta+2', () => setActiveTab('search'), { preventDefault: true })
  useHotkeys('meta+o', () => handleImport(), { preventDefault: true })
  useHotkeys('meta+t', () => setTheme(theme === 'dark' ? 'light' : 'dark'), {
    preventDefault: true,
  })

  const handleFilesImport = useCallback(async (filePaths: string[]) => {
    try {
      console.log('Importing files:', filePaths)

      // Get file information (name and size)
      const filesInfo: FileInfo[] = await Promise.all(
        filePaths.map(async (path) => {
          const fileStats = await stat(path)
          const fileName = path.split('/').pop() || path.split('\\').pop() || path
          return {
            path,
            name: fileName,
            size: fileStats.size,
          }
        })
      )

      setFilesToImport(filesInfo)
      setImportDialogOpen(true)
    } catch (error) {
      console.error('Failed to get file information:', error)
      toast.error(`Failed to read files: ${error}`)
    }
  }, [])

  // Listen for file events (both drag-drop and file associations)
  useEffect(() => {
    // Listen for drag and drop events
    const unlistenDrop = listen<string[]>('file-opened', (event) => {
      console.log('File drop event:', event)
      const droppedFiles = event.payload

      // Filter for .json and .jsonl files only
      const validFiles = droppedFiles.filter((file) => {
        if (typeof file !== 'string') return false
        const extension = file.toLowerCase().split('.').pop()
        return extension === 'json' || extension === 'jsonl'
      })

      if (validFiles.length > 0) {
        handleFilesImport(validFiles)
      } else {
        toast.warning('Please drop .json or .jsonl files only')
      }
    })

    return () => {
      unlistenDrop.then((fn) => fn())
    }
  }, [handleFilesImport])

  const handleImport = async () => {
    try {
      // Open file dialog to select files
      const files = await open({
        multiple: true,
        directory: false,
        filters: [
          {
            name: 'Chat Sessions',
            extensions: ['json', 'jsonl'],
          },
        ],
      })

      if (!files) {
        console.log('No files selected')
        return
      }

      // Handle both single file and multiple files
      const filePaths = Array.isArray(files) ? files : [files]

      // Reuse the same import function
      await handleFilesImport(filePaths)
    } catch (error) {
      console.error('Failed to import files:', error)
      toast.error(`Failed to import: ${error}`)
    }
  }

  const confirmImport = async () => {
    const filePaths = filesToImport.map((file) => file.path)
    console.log('Confirming import for:', filePaths)

    setImportDialogOpen(false)

    // Use promise-based toast for better control
    toast.promise(
      invoke<{
        total_files: number
        successful_imports: number
        failed_imports: number
        total_sessions_imported: number
        total_messages_imported: number
        results: Array<{
          file_path: string
          sessions_imported: number
          messages_imported: number
          success: boolean
          error?: string
        }>
      }>('import_sessions', { filePaths }),
      {
        loading: 'Importing sessions...',
        success: async (response) => {
          // Refresh the session list and select first session
          setRefreshTrigger((prev) => prev + 1)

          // Fetch the first session and select it
          try {
            const { getSessions } = await import('@/lib/api')
            const sessions = await getSessions(1, 1, provider)
            if (sessions.length > 0) {
              setSelectedSession(sessions[0].id)
            }
          } catch (error) {
            console.error('Failed to fetch first session:', error)
          }

          if (response.failed_imports > 0) {
            const failedFiles = response.results
              .filter((r) => !r.success)
              .map((r) => r.file_path.split('/').pop())
              .join(', ')

            return `Imported ${response.successful_imports}/${response.total_files} files. Failed: ${failedFiles}`
          }

          return `Successfully imported ${response.total_sessions_imported} sessions (${response.total_messages_imported} messages) from ${response.total_files} file(s)`
        },
        error: (error) => {
          console.error('Failed to import files:', error)
          return `Failed to import: ${error}`
        },
      }
    )
  }

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return `${Math.round((bytes / k ** i) * 100) / 100} ${sizes[i]}`
  }

  return (
    <TooltipProvider>
      <div className="flex h-screen bg-background">
        {/* Sidebar */}
        <div className="w-80 border-r border-border bg-card flex flex-col h-full">
          <div className="p-4 border-b border-border flex-shrink-0">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-2">
                <History className="w-6 h-6 text-primary" />
                <h1 className="text-xl font-semibold text-foreground">RetroChat</h1>
              </div>
              <div className="flex items-center gap-2">
                <ThemeToggle />
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button variant="outline" size="sm" onClick={handleImport} className="gap-2">
                      <Upload className="w-4 h-4" />
                      Import
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    <div className="flex items-center gap-2">
                      <p>Import chat sessions from JSON or JSONL files</p>
                      <KbdGroup>
                        <Kbd>⌘</Kbd>
                        <Kbd>O</Kbd>
                      </KbdGroup>
                    </div>
                  </TooltipContent>
                </Tooltip>
              </div>
            </div>

            <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
              <TabsList className="grid w-full grid-cols-2">
                <TabsTrigger value="sessions">
                  <Tooltip delayDuration={300}>
                    <TooltipTrigger asChild>
                      <span>Sessions</span>
                    </TooltipTrigger>
                    <TooltipContent>
                      <div className="flex items-center gap-2">
                        <p>Browse all chat sessions</p>
                        <KbdGroup>
                          <Kbd>⌘</Kbd>
                          <Kbd>1</Kbd>
                        </KbdGroup>
                      </div>
                    </TooltipContent>
                  </Tooltip>
                </TabsTrigger>
                <TabsTrigger value="search">
                  <Tooltip delayDuration={300}>
                    <TooltipTrigger asChild>
                      <span>Search</span>
                    </TooltipTrigger>
                    <TooltipContent>
                      <div className="flex items-center gap-2">
                        <p>Search through messages</p>
                        <KbdGroup>
                          <Kbd>⌘</Kbd>
                          <Kbd>2</Kbd>
                        </KbdGroup>
                      </div>
                    </TooltipContent>
                  </Tooltip>
                </TabsTrigger>
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
                onImport={handleImport}
                refreshTrigger={refreshTrigger}
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

      {/* Import Files Dialog */}
      <Dialog open={importDialogOpen} onOpenChange={setImportDialogOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Import Chat Sessions</DialogTitle>
            <DialogDescription>
              Review the files you're about to import. Click confirm to proceed.
            </DialogDescription>
          </DialogHeader>

          <ScrollArea className="max-h-[400px] pr-4">
            <div className="space-y-2">
              {filesToImport.map((file) => (
                <div
                  key={file.path}
                  className="flex items-center gap-3 p-3 rounded-lg border border-border bg-card"
                >
                  <FileText className="w-5 h-5 text-primary shrink-0" />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-foreground truncate">{file.name}</p>
                    <p className="text-xs text-muted-foreground">{formatFileSize(file.size)}</p>
                  </div>
                </div>
              ))}
            </div>
          </ScrollArea>

          <DialogFooter>
            <Button variant="outline" onClick={() => setImportDialogOpen(false)}>
              Cancel
            </Button>
            <Button onClick={confirmImport}>
              Import {filesToImport.length} {filesToImport.length === 1 ? 'file' : 'files'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </TooltipProvider>
  )
}
