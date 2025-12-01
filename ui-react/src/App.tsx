import { Toaster } from 'sonner'
import { SessionManager } from './components/session-manager'
import { ThemeProvider } from './components/theme-provider'

function App() {
  // Initialize updater (checks for updates on startup)
  // useUpdater() - temporarily disabled

  return (
    <ThemeProvider attribute="class" defaultTheme="system" enableSystem>
      <SessionManager />
      <Toaster richColors />
    </ThemeProvider>
  )
}

export default App
