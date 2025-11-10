// Import Tauri API - Try multiple paths for compatibility
let invoke;
if (window.__TAURI_INTERNALS__) {
    invoke = window.__TAURI_INTERNALS__.invoke;
} else if (window.__TAURI__ && window.__TAURI__.core) {
    invoke = window.__TAURI__.core.invoke;
} else if (window.__TAURI__ && window.__TAURI__.tauri) {
    invoke = window.__TAURI__.tauri.invoke;
} else {
    console.error('Tauri API not found!');
}

// Application State
let currentPage = 1;
let currentProvider = '';
let currentSessionId = null;

// DOM Elements
const sessionsList = document.getElementById('sessionsList');
const sessionDetail = document.getElementById('sessionDetail');
const searchInput = document.getElementById('searchInput');
const searchBtn = document.getElementById('searchBtn');
const providerFilter = document.getElementById('providerFilter');
const prevPageBtn = document.getElementById('prevPage');
const nextPageBtn = document.getElementById('nextPage');
const pageInfo = document.getElementById('pageInfo');
const searchModal = document.getElementById('searchModal');
const closeModalBtn = document.getElementById('closeModal');
const searchResults = document.getElementById('searchResults');

// Initialize the application
async function init() {
    console.log('Initializing application...');
    console.log('Tauri invoke function:', typeof invoke);

    if (!invoke) {
        sessionsList.innerHTML = '<p class="error">Tauri API not available. Please check console for errors.</p>';
        return;
    }

    await loadSessions();
    setupEventListeners();
}

// Setup event listeners
function setupEventListeners() {
    searchBtn.addEventListener('click', performSearch);
    searchInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            performSearch();
        }
    });

    providerFilter.addEventListener('change', async () => {
        currentProvider = providerFilter.value;
        currentPage = 1;
        await loadSessions();
    });

    prevPageBtn.addEventListener('click', async () => {
        if (currentPage > 1) {
            currentPage--;
            await loadSessions();
        }
    });

    nextPageBtn.addEventListener('click', async () => {
        currentPage++;
        await loadSessions();
    });

    closeModalBtn.addEventListener('click', () => {
        searchModal.classList.add('hidden');
    });

    // Close modal when clicking outside
    searchModal.addEventListener('click', (e) => {
        if (e.target === searchModal) {
            searchModal.classList.add('hidden');
        }
    });
}

// Load sessions from backend
async function loadSessions() {
    try {
        sessionsList.innerHTML = '<p class="loading">Loading sessions...</p>';

        const sessions = await invoke('get_sessions', {
            page: currentPage,
            pageSize: 20,
            provider: currentProvider || null,
        });

        if (sessions.length === 0) {
            sessionsList.innerHTML = '<p class="loading">No sessions found.</p>';
            prevPageBtn.disabled = currentPage <= 1;
            nextPageBtn.disabled = true;
            return;
        }

        renderSessions(sessions);
        updatePaginationButtons(sessions.length);
    } catch (error) {
        console.error('Failed to load sessions:', error);
        sessionsList.innerHTML = `<p class="error">Failed to load sessions: ${error}</p>`;
    }
}

// Render sessions list
function renderSessions(sessions) {
    sessionsList.innerHTML = sessions
        .map(
            (session) => `
        <div class="session-item ${session.id === currentSessionId ? 'active' : ''}"
             data-session-id="${session.id}">
            <div class="session-header">
                <span class="provider">${session.provider}</span>
                <span class="date">${formatDate(session.updated_at)}</span>
            </div>
            <div class="project">${session.project_name || 'Unnamed Project'}</div>
            <div class="meta">${session.message_count} messages</div>
        </div>
    `
        )
        .join('');

    // Add click handlers
    document.querySelectorAll('.session-item').forEach((item) => {
        item.addEventListener('click', () => {
            const sessionId = item.getAttribute('data-session-id');
            loadSessionDetail(sessionId);
        });
    });
}

// Update pagination buttons
function updatePaginationButtons(sessionCount) {
    prevPageBtn.disabled = currentPage <= 1;
    nextPageBtn.disabled = sessionCount < 20;
    pageInfo.textContent = `Page ${currentPage}`;
}

// Load session details
async function loadSessionDetail(sessionId) {
    try {
        currentSessionId = sessionId;

        // Update active state in session list
        document.querySelectorAll('.session-item').forEach((item) => {
            item.classList.remove('active');
        });
        document
            .querySelector(`[data-session-id="${sessionId}"]`)
            ?.classList.add('active');

        sessionDetail.innerHTML = '<p class="loading">Loading session...</p>';

        const session = await invoke('get_session_detail', { sessionId });
        renderSessionDetail(session);
    } catch (error) {
        console.error('Failed to load session detail:', error);
        sessionDetail.innerHTML = `<p class="error">Failed to load session: ${error}</p>`;
    }
}

// Render session detail
function renderSessionDetail(session) {
    const html = `
        <div class="detail-header">
            <h2>${session.project_name || 'Unnamed Project'}</h2>
            <div class="meta">
                <span><span class="label">Provider:</span> ${session.provider}</span>
                <span><span class="label">Messages:</span> ${session.messages.length}</span>
                <span><span class="label">Created:</span> ${formatDate(session.created_at)}</span>
                <span><span class="label">Updated:</span> ${formatDate(session.updated_at)}</span>
            </div>
        </div>
        <div class="messages">
            ${session.messages
                .map(
                    (msg) => `
                <div class="message ${msg.role.toLowerCase()}">
                    <div class="message-header">
                        <span class="message-role">${msg.role}</span>
                        <span class="message-time">${formatDateTime(msg.timestamp)}</span>
                    </div>
                    <div class="message-content">${escapeHtml(msg.content)}</div>
                </div>
            `
                )
                .join('')}
        </div>
    `;

    sessionDetail.innerHTML = html;
}

// Perform search
async function performSearch() {
    const query = searchInput.value.trim();
    if (!query) {
        return;
    }

    try {
        searchResults.innerHTML = '<p class="loading">Searching...</p>';
        searchModal.classList.remove('hidden');

        const results = await invoke('search_messages', {
            query,
            limit: 50,
        });

        if (results.length === 0) {
            searchResults.innerHTML = '<p class="loading">No results found.</p>';
            return;
        }

        renderSearchResults(results);
    } catch (error) {
        console.error('Search failed:', error);
        searchResults.innerHTML = `<p class="error">Search failed: ${error}</p>`;
    }
}

// Render search results
function renderSearchResults(results) {
    searchResults.innerHTML = results
        .map(
            (result) => `
        <div class="search-result-item" data-session-id="${result.session_id}">
            <div class="result-header">
                <span class="role">${result.role}</span>
                <span class="provider">${result.provider}</span>
            </div>
            <div class="content">${escapeHtml(result.content)}</div>
            <div class="time">${formatDateTime(result.timestamp)}</div>
        </div>
    `
        )
        .join('');

    // Add click handlers to navigate to session
    document.querySelectorAll('.search-result-item').forEach((item) => {
        item.addEventListener('click', () => {
            const sessionId = item.getAttribute('data-session-id');
            searchModal.classList.add('hidden');
            loadSessionDetail(sessionId);
        });
    });
}

// Utility: Format date
function formatDate(dateString) {
    const date = new Date(dateString);
    const now = new Date();
    const diff = now - date;

    const days = Math.floor(diff / (1000 * 60 * 60 * 24));
    const hours = Math.floor(diff / (1000 * 60 * 60));
    const minutes = Math.floor(diff / (1000 * 60));

    if (minutes < 1) return 'Just now';
    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    if (days < 7) return `${days}d ago`;

    return date.toLocaleDateString();
}

// Utility: Format date and time
function formatDateTime(dateString) {
    const date = new Date(dateString);
    return date.toLocaleString();
}

// Utility: Escape HTML
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Start the application when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
