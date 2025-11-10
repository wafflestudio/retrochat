import { invoke } from '@tauri-apps/api/core'
import type { Analytics, AnalyticsRequest, SearchResult, Session, SessionWithMessages } from '@/types'

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

/**
 * Create a new analysis request for a session
 * @param sessionId - Session UUID
 * @param customPrompt - Optional custom prompt for analysis
 * @returns Created analytics request
 */
export async function createAnalysis(
  sessionId: string,
  customPrompt?: string
): Promise<AnalyticsRequest> {
  return await invoke('create_analysis', { sessionId, customPrompt })
}

/**
 * Execute an analysis request
 * @param requestId - Analytics request ID
 * @returns Session ID that was analyzed
 */
export async function runAnalysis(requestId: string): Promise<string> {
  return await invoke('run_analysis', { requestId })
}

/**
 * Get the status of an analysis request
 * @param requestId - Analytics request ID
 * @returns Analytics request status
 */
export async function getAnalysisStatus(requestId: string): Promise<AnalyticsRequest> {
  return await invoke('get_analysis_status', { requestId })
}

/**
 * Get the result of a completed analysis
 * @param requestId - Analytics request ID
 * @returns Analytics result or null if not completed
 */
export async function getAnalysisResult(requestId: string): Promise<Analytics | null> {
  return await invoke('get_analysis_result', { requestId })
}

/**
 * List analysis requests
 * @param sessionId - Optional session ID to filter by
 * @param limit - Maximum number of results
 * @returns List of analytics requests
 */
export async function listAnalyses(
  sessionId?: string,
  limit?: number
): Promise<AnalyticsRequest[]> {
  return await invoke('list_analyses', { sessionId, limit })
}

/**
 * Cancel a pending or running analysis request
 * @param requestId - Analytics request ID
 */
export async function cancelAnalysis(requestId: string): Promise<void> {
  return await invoke('cancel_analysis', { requestId })
}
