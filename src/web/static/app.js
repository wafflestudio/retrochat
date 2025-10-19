// API Base URL
const API_BASE = '/api';

// State
let currentPage = 1;
let currentProvider = '';
let currentSortBy = 'start_time';
let isSearchMode = false;

// Elements
const sessionsView = document.getElementById('sessions-view');
const searchResultsView = document.getElementById('search-results-view');
const sessionsList = document.getElementById('sessions-list');
const searchResults = document.getElementById('search-results');
const searchInput = document.getElementById('search-input');
const searchBtn = document.getElementById('search-btn');
const clearSearchBtn = document.getElementById('clear-search-btn');
const providerFilter = document.getElementById('provider-filter');
const sortBySelect = document.getElementById('sort-by');
const prevPageBtn = document.getElementById('prev-page');
const nextPageBtn = document.getElementById('next-page');
const pageInfo = document.getElementById('page-info');
const refreshBtn = document.getElementById('refresh-btn');
const modal = document.getElementById('session-modal');
const closeModalBtn = document.getElementById('close-modal');
const modalTitle = document.getElementById('modal-title');
const modalBody = document.getElementById('modal-body');
const totalSessionsEl = document.getElementById('total-sessions');
const statusEl = document.getElementById('status');

// Initialize
async function init() {
    checkHealth();
    loadSessions();
    setupEventListeners();
}

// Check API health
async function checkHealth() {
    try {
        const response = await fetch(`${API_BASE}/health`);
        const data = await response.json();
        if (data.status === 'ok') {
            updateStatus('Ready', true);
        }
    } catch (error) {
        updateStatus('Error', false);
        console.error('Health check failed:', error);
    }
}

// Update status indicator
function updateStatus(text, isOk) {
    const statusDot = statusEl.querySelector('.status-dot');
    // Remove all text nodes and re-add the dot and text
    statusEl.textContent = '';
    statusEl.appendChild(statusDot);
    statusEl.appendChild(document.createTextNode(` ${text}`));

    if (isOk) {
        statusDot.style.background = 'var(--success)';
    } else {
        statusDot.style.background = 'var(--error)';
        statusDot.style.animation = 'none';
    }
}

// Setup event listeners
function setupEventListeners() {
    searchBtn.addEventListener('click', handleSearch);
    clearSearchBtn.addEventListener('click', clearSearch);
    searchInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') handleSearch();
    });

    providerFilter.addEventListener('change', () => {
        currentProvider = providerFilter.value;
        currentPage = 1;
        loadSessions();
    });

    sortBySelect.addEventListener('change', () => {
        currentSortBy = sortBySelect.value;
        currentPage = 1;
        loadSessions();
    });

    prevPageBtn.addEventListener('click', () => {
        if (currentPage > 1) {
            currentPage--;
            loadSessions();
        }
    });

    nextPageBtn.addEventListener('click', () => {
        currentPage++;
        loadSessions();
    });

    refreshBtn.addEventListener('click', () => {
        if (isSearchMode) {
            handleSearch();
        } else {
            loadSessions();
        }
    });

    closeModalBtn.addEventListener('click', closeModal);
    modal.addEventListener('click', (e) => {
        if (e.target === modal) closeModal();
    });

    // Close modal on Escape key
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape' && modal.classList.contains('active')) {
            closeModal();
        }
    });
}

// Load sessions
async function loadSessions() {
    try {
        sessionsList.innerHTML = '<div class="loading">Loading sessions...</div>';

        const params = new URLSearchParams({
            page: currentPage,
            page_size: 20,
            sort_by: currentSortBy,
            sort_order: 'desc'
        });

        if (currentProvider) {
            params.append('provider', currentProvider);
        }

        const response = await fetch(`${API_BASE}/sessions?${params}`);
        const data = await response.json();

        renderSessions(data);
        updatePagination(data);
        totalSessionsEl.textContent = data.total_count;
    } catch (error) {
        console.error('Error loading sessions:', error);
        sessionsList.innerHTML = '<div class="empty-state"><h3>Error loading sessions</h3><p>Please try again later.</p></div>';
    }
}

// Render sessions
function renderSessions(data) {
    if (!data.sessions || data.sessions.length === 0) {
        sessionsList.innerHTML = '<div class="empty-state"><h3>No sessions found</h3><p>Try importing some chat history first.</p></div>';
        return;
    }

    sessionsList.innerHTML = data.sessions.map(session => `
        <div class="session-card" onclick="loadSessionDetail('${session.session_id}')">
            <div class="session-header">
                <div class="session-title">
                    <span class="provider-badge ${session.provider.toLowerCase()}">${session.provider}</span>
                    ${session.project ? `<span class="project-name">${session.project}</span>` : ''}
                </div>
                <div class="session-meta">
                    <span class="session-meta-item">
                        ${session.message_count} messages
                    </span>
                    ${session.total_tokens ? `<span class="session-meta-item">${session.total_tokens.toLocaleString()} tokens</span>` : ''}
                </div>
            </div>
            <div class="session-meta">
                <span class="session-meta-item">
                    ${formatDate(session.start_time)}
                </span>
                ${session.has_retrospection ? '<span class="session-meta-item">Has Retrospection</span>' : ''}
            </div>
            <div class="session-preview">${escapeHtml(session.first_message_preview)}</div>
        </div>
    `).join('');
}

// Update pagination
function updatePagination(data) {
    pageInfo.textContent = `Page ${data.page} of ${data.total_pages}`;
    prevPageBtn.disabled = data.page <= 1;
    nextPageBtn.disabled = data.page >= data.total_pages;
}

// Handle search
async function handleSearch() {
    const query = searchInput.value.trim();
    if (!query) return;

    try {
        isSearchMode = true;
        sessionsView.style.display = 'none';
        searchResultsView.style.display = 'block';
        clearSearchBtn.style.display = 'inline-block';

        searchResults.innerHTML = '<div class="loading">Searching...</div>';

        const params = new URLSearchParams({
            query: query,
            page: 1,
            page_size: 50
        });

        const response = await fetch(`${API_BASE}/search?${params}`);
        const data = await response.json();

        renderSearchResults(data);
    } catch (error) {
        console.error('Error searching:', error);
        searchResults.innerHTML = '<div class="empty-state"><h3>Search failed</h3><p>Please try again.</p></div>';
    }
}

// Render search results
function renderSearchResults(data) {
    const searchStats = document.getElementById('search-stats');
    searchStats.textContent = `Found ${data.total_count} results in ${data.search_duration_ms}ms`;

    if (!data.results || data.results.length === 0) {
        searchResults.innerHTML = '<div class="empty-state"><h3>No results found</h3><p>Try a different search query.</p></div>';
        return;
    }

    searchResults.innerHTML = data.results.map(result => `
        <div class="search-result" onclick="loadSessionDetail('${result.session_id}')">
            <div class="search-result-header">
                <span class="provider-badge ${result.provider.toLowerCase()}">${result.provider}</span>
                <div class="search-result-meta">
                    <span>${formatDate(result.timestamp)}</span>
                    <span class="message-role">${result.message_role}</span>
                </div>
            </div>
            ${result.project ? `<div class="project-name" style="margin-bottom: 8px;">${result.project}</div>` : ''}
            <div class="search-result-content">${highlightSearchTerm(escapeHtml(result.content_snippet), searchInput.value)}</div>
        </div>
    `).join('');
}

// Clear search
function clearSearch() {
    isSearchMode = false;
    searchInput.value = '';
    sessionsView.style.display = 'block';
    searchResultsView.style.display = 'none';
    clearSearchBtn.style.display = 'none';
    loadSessions();
}

// Load session detail
async function loadSessionDetail(sessionId) {
    try {
        modal.classList.add('active');
        modalTitle.textContent = 'Session Details';
        modalBody.innerHTML = '<div class="loading">Loading session details...</div>';

        const response = await fetch(`${API_BASE}/sessions/${sessionId}`);
        const data = await response.json();

        renderSessionDetail(data);
    } catch (error) {
        console.error('Error loading session detail:', error);
        modalBody.innerHTML = '<div class="empty-state"><h3>Error loading session</h3><p>Please try again.</p></div>';
    }
}

// Render session detail
function renderSessionDetail(data) {
    const { session, messages } = data;

    modalTitle.textContent = `${session.provider} Session`;

    modalBody.innerHTML = `
        <div class="session-detail">
            <div class="detail-section">
                <h3>Session Information</h3>
                <div class="detail-grid">
                    <div class="detail-item">
                        <span class="detail-label">Provider</span>
                        <span class="detail-value">
                            <span class="provider-badge ${session.provider.toLowerCase()}">${session.provider}</span>
                        </span>
                    </div>
                    ${session.project_name ? `
                        <div class="detail-item">
                            <span class="detail-label">Project</span>
                            <span class="detail-value">${session.project_name}</span>
                        </div>
                    ` : ''}
                    <div class="detail-item">
                        <span class="detail-label">Messages</span>
                        <span class="detail-value">${session.message_count}</span>
                    </div>
                    ${session.token_count ? `
                        <div class="detail-item">
                            <span class="detail-label">Tokens</span>
                            <span class="detail-value">${session.token_count.toLocaleString()}</span>
                        </div>
                    ` : ''}
                    <div class="detail-item">
                        <span class="detail-label">Started</span>
                        <span class="detail-value">${formatDate(session.start_time)}</span>
                    </div>
                    ${session.end_time ? `
                        <div class="detail-item">
                            <span class="detail-label">Ended</span>
                            <span class="detail-value">${formatDate(session.end_time)}</span>
                        </div>
                    ` : ''}
                    ${session.file_path ? `
                        <div class="detail-item" style="grid-column: 1 / -1;">
                            <span class="detail-label">File Path</span>
                            <span class="detail-value" style="font-family: monospace; font-size: 0.85rem; word-break: break-all;">${escapeHtml(session.file_path)}</span>
                        </div>
                    ` : ''}
                </div>
            </div>

            <div class="detail-section">
                <h3>Messages (${messages.length})</h3>
                <div class="messages">
                    ${messages.map(msg => `
                        <div class="message ${msg.role}">
                            <div class="message-header">
                                <span class="message-role">${msg.role}</span>
                                <span class="message-time">${formatTime(msg.timestamp)}</span>
                            </div>
                            <div class="message-content">${escapeHtml(msg.content)}</div>
                            ${msg.tool_uses && msg.tool_uses.length > 0 ? `
                                <div style="margin-top: 12px; padding-top: 12px; border-top: 1px solid var(--border);">
                                    <strong>Tools Used:</strong>
                                    ${msg.tool_uses.map(tool => `<div style="margin-top: 4px;">${tool.name}</div>`).join('')}
                                </div>
                            ` : ''}
                        </div>
                    `).join('')}
                </div>
            </div>
        </div>
    `;
}

// Close modal
function closeModal() {
    modal.classList.remove('active');
}

// Utility functions
function formatDate(dateString) {
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now - date;
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;

    return date.toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric'
    });
}

function formatTime(dateString) {
    const date = new Date(dateString);
    return date.toLocaleString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
    });
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function highlightSearchTerm(text, term) {
    if (!term) return text;
    const regex = new RegExp(`(${escapeRegex(term)})`, 'gi');
    return text.replace(regex, '<mark style="background: var(--warning); color: var(--background); padding: 2px 4px; border-radius: 3px;">$1</mark>');
}

function escapeRegex(string) {
    return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

// Start the app
init();
