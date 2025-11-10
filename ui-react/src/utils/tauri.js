import { invoke } from '@tauri-apps/api/core';

/**
 * Get paginated list of chat sessions
 * @param {number} page - Page number
 * @param {number} pageSize - Items per page
 * @param {string|null} provider - Filter by provider
 * @returns {Promise<Array>} List of sessions
 */
export async function getSessions(page = 1, pageSize = 20, provider = null) {
  return await invoke('get_sessions', {
    page,
    pageSize,
    provider,
  });
}

/**
 * Get detailed information about a specific session
 * @param {string} sessionId - Session UUID
 * @returns {Promise<Object>} Session detail with messages
 */
export async function getSessionDetail(sessionId) {
  return await invoke('get_session_detail', { sessionId });
}

/**
 * Search messages by content
 * @param {string} query - Search query
 * @param {number} limit - Maximum results
 * @returns {Promise<Array>} Search results
 */
export async function searchMessages(query, limit = 50) {
  return await invoke('search_messages', { query, limit });
}

/**
 * Get list of available providers
 * @returns {Promise<Array<string>>} List of providers
 */
export async function getProviders() {
  return await invoke('get_providers');
}
