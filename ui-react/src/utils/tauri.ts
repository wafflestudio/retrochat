import { invoke } from '@tauri-apps/api/core'
import type { SearchResult, Session, SessionWithMessages } from '@/types'

/**
 * Get paginated list of chat sessions
 * @param page - Page number
 * @param pageSize - Items per page
 * @param provider - Filter by provider
 * @returns List of sessions
 */
export async function getSessions(
  page: number = 1,
  pageSize: number = 20,
  provider: string | null = null
): Promise<Session[]> {
  return await invoke('get_sessions', {
    page,
    pageSize,
    provider,
  })
}

/**
 * Get detailed information about a specific session
 * @param sessionId - Session UUID
 * @returns Session detail with messages
 */
export async function getSessionDetail(sessionId: string): Promise<SessionWithMessages> {
  return await invoke('get_session_detail', { sessionId })
}

/**
 * Search messages by content
 * @param query - Search query
 * @param limit - Maximum results
 * @returns Search results
 */
export async function searchMessages(query: string, limit: number = 50): Promise<SearchResult[]> {
  return await invoke('search_messages', { query, limit })
}

/**
 * Get list of available providers
 * @returns List of providers
 */
export async function getProviders(): Promise<string[]> {
  return await invoke('get_providers')
}
