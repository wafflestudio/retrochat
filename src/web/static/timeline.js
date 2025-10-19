// Timeline Module
const Timeline = {
    // State
    currentMessages: [],
    displayMode: 'compact', // 'compact' or 'full'

    // Elements
    elements: {
        output: null,
        stats: null,
        sinceInput: null,
        untilInput: null,
        providerSelect: null,
        roleSelect: null,
        modeSelect: null,
        queryBtn: null,
        copyBtn: null,
        exportBtn: null,
        charCount: null,
        tokenCount: null,
    },

    // Initialize
    init() {
        this.elements = {
            output: document.getElementById('timeline-output'),
            stats: document.getElementById('timeline-stats'),
            sinceInput: document.getElementById('timeline-since'),
            untilInput: document.getElementById('timeline-until'),
            providerSelect: document.getElementById('timeline-provider'),
            roleSelect: document.getElementById('timeline-role'),
            modeSelect: document.getElementById('timeline-mode'),
            queryBtn: document.getElementById('timeline-query-btn'),
            copyBtn: document.getElementById('timeline-copy-btn'),
            exportBtn: document.getElementById('timeline-export-btn'),
            charCount: document.getElementById('timeline-char-count'),
            tokenCount: document.getElementById('timeline-token-count'),
        };

        this.setupEventListeners();
    },

    // Setup event listeners
    setupEventListeners() {
        // Query button
        if (this.elements.queryBtn) {
            this.elements.queryBtn.addEventListener('click', () => this.query());
        }

        // Quick filter buttons - use event delegation for dynamically loaded content
        document.addEventListener('click', (e) => {
            if (e.target.classList.contains('quick-filter-btn')) {
                e.preventDefault();
                const since = e.target.dataset.since;
                this.elements.sinceInput.value = since;
                this.elements.untilInput.value = 'now';
            }
        });

        // Copy all button
        if (this.elements.copyBtn) {
            this.elements.copyBtn.addEventListener('click', () => this.copyAll());
        }

        // Export JSONL button
        if (this.elements.exportBtn) {
            this.elements.exportBtn.addEventListener('click', () => this.exportJSONL());
        }

        // Enter key in inputs
        [this.elements.sinceInput, this.elements.untilInput].forEach(input => {
            if (input) {
                input.addEventListener('keypress', (e) => {
                    if (e.key === 'Enter') this.query();
                });
            }
        });

        // Mode change
        if (this.elements.modeSelect) {
            this.elements.modeSelect.addEventListener('change', (e) => {
                this.displayMode = e.target.value;
                if (this.currentMessages.length > 0) {
                    // Re-render with new mode
                    this.renderMessages({ messages: this.currentMessages, total_count: this.currentMessages.length });
                }
            });
        }
    },

    // Query timeline
    async query() {
        const params = new URLSearchParams();

        const since = this.elements.sinceInput.value.trim();
        const until = this.elements.untilInput.value.trim();
        const provider = this.elements.providerSelect.value;
        const role = this.elements.roleSelect.value;
        const format = this.elements.modeSelect.value;

        if (since) params.append('since', since);
        if (until) params.append('until', until);
        if (provider) params.append('provider', provider);
        if (role) params.append('role', role);
        if (format) params.append('format', format);

        try {
            this.showLoading();
            const response = await fetch(`/api/timeline?${params}`);

            if (!response.ok) {
                const error = await response.json();
                throw new Error(error.error || 'Query failed');
            }

            const data = await response.json();
            this.currentMessages = data.messages;
            this.renderMessages(data);
            this.updateStats(data);

            // Enable action buttons
            this.elements.copyBtn.disabled = data.messages.length === 0;
            this.elements.exportBtn.disabled = data.messages.length === 0;
        } catch (error) {
            console.error('Timeline query failed:', error);
            this.showError(error.message);
        }
    },

    // Show loading state
    showLoading() {
        this.elements.output.innerHTML = '<div class="loading">Querying timeline...</div>';
    },

    // Show error
    showError(message) {
        this.elements.output.innerHTML = `
            <div class="error-message">
                <strong>Error:</strong> ${message}
            </div>
        `;
    },

    // Update stats
    updateStats(data) {
        const fromStr = data.time_range.from ? new Date(data.time_range.from).toLocaleString() : 'beginning';
        const toStr = data.time_range.to ? new Date(data.time_range.to).toLocaleString() : 'now';

        this.elements.stats.textContent = `${data.total_count} messages (${fromStr} → ${toStr})`;
    },

    // Render messages
    renderMessages(data) {
        if (data.messages.length === 0) {
            this.elements.output.innerHTML = '<div class="timeline-empty">No messages found</div>';
            this.updateCharTokenCount(0, 0);
            return;
        }

        const messagesHtml = data.messages.map(msg => this.renderMessage(msg)).join('');
        this.elements.output.innerHTML = messagesHtml;

        // Calculate total chars and tokens
        const totalChars = this.currentMessages.reduce((sum, msg) => sum + msg.content.length, 0);
        const totalTokens = Math.ceil(totalChars / 4); // 4 chars ≈ 1 token
        this.updateCharTokenCount(totalChars, totalTokens);
    },

    // Update char and token count display
    updateCharTokenCount(chars, tokens) {
        if (this.elements.charCount) {
            this.elements.charCount.textContent = `${chars.toLocaleString()} chars`;
        }
        if (this.elements.tokenCount) {
            this.elements.tokenCount.textContent = `~${tokens.toLocaleString()} tokens`;
        }
    },

    // Render single message
    renderMessage(msg) {
        const timestamp = new Date(msg.timestamp);
        const timeStr = this.formatTime(timestamp);
        const providerIcon = this.getProviderIcon(msg.provider);
        const projectStr = msg.project || 'None';

        // Apply truncation based on display mode
        const content = this.displayMode === 'full'
            ? msg.content
            : this.truncateContent(msg.content);

        const displayText = this.displayMode === 'full' ? content : content.text;
        const isTruncated = this.displayMode === 'compact' && content.isTruncated;

        return `
            <div class="timeline-message" data-message-id="${msg.message_id}">
                <div class="timeline-message-header">
                    <span class="timeline-time">${timeStr}</span>
                    <span class="timeline-role">[${msg.role.padEnd(9)}]</span>
                    <span class="timeline-provider">${providerIcon} ${msg.provider}</span>
                    <span class="timeline-project">(${projectStr})</span>
                    <button class="timeline-copy-msg" data-message-id="${msg.message_id}" title="Copy message">Copy</button>
                </div>
                <div class="timeline-message-content">${this.escapeHtml(displayText)}</div>
                ${isTruncated ? `<div class="timeline-truncated">[${content.hiddenChars} more chars]</div>` : ''}
            </div>
        `;
    },

    // Format timestamp
    formatTime(date) {
        const month = String(date.getMonth() + 1).padStart(2, '0');
        const day = String(date.getDate()).padStart(2, '0');
        const hours = String(date.getHours()).padStart(2, '0');
        const minutes = String(date.getMinutes()).padStart(2, '0');
        return `${month}-${day} ${hours}:${minutes}`;
    },

    // Get provider icon
    getProviderIcon(provider) {
        const icons = {
            'Claude': '🟠',
            'Gemini': '🔵',
            'Cursor': '🔷',
            'Codex': '⚪',
        };
        return icons[provider] || '⚫';
    },

    // Truncate content (head 400 + tail 200)
    truncateContent(content, headChars = 400, tailChars = 200) {
        if (content.length <= headChars + tailChars) {
            return { text: content, isTruncated: false, hiddenChars: 0 };
        }

        const head = content.substring(0, headChars);
        const tail = content.substring(content.length - tailChars);
        const hiddenChars = content.length - headChars - tailChars;

        return {
            text: `${head}\n...\n${tail}`,
            isTruncated: true,
            hiddenChars: hiddenChars,
        };
    },

    // Escape HTML
    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    },

    // Copy all messages
    async copyAll() {
        if (this.currentMessages.length === 0) return;

        const text = this.formatMessagesForCopy(this.currentMessages);

        try {
            await navigator.clipboard.writeText(text);
            this.showCopyFeedback(this.elements.copyBtn);
        } catch (error) {
            console.error('Copy failed:', error);
            alert('Failed to copy to clipboard');
        }
    },

    // Format messages for copying
    formatMessagesForCopy(messages) {
        const since = this.elements.sinceInput.value || 'beginning';
        const until = this.elements.untilInput.value || 'now';

        let text = '=== RetroChat Timeline Export ===\n';
        text += `Time Range: ${since} to ${until}\n`;
        text += `Total: ${messages.length} messages\n\n`;
        text += '---\n\n';

        messages.forEach(msg => {
            const timestamp = new Date(msg.timestamp);
            const timeStr = this.formatTime(timestamp);
            const projectStr = msg.project || 'None';

            text += `${timeStr} [${msg.role}] ${msg.provider} (${projectStr})\n`;
            text += `${msg.content}\n\n`;
        });

        return text;
    },

    // Show copy feedback
    showCopyFeedback(button) {
        const originalText = button.textContent;
        button.textContent = '✓ Copied!';
        button.disabled = true;

        setTimeout(() => {
            button.textContent = originalText;
            button.disabled = false;
        }, 2000);
    },

    // Export as JSONL
    exportJSONL() {
        if (this.currentMessages.length === 0) return;

        const jsonl = this.currentMessages.map(msg => JSON.stringify(msg)).join('\n');
        const blob = new Blob([jsonl], { type: 'application/jsonl' });
        const url = URL.createObjectURL(blob);

        const a = document.createElement('a');
        a.href = url;
        a.download = `timeline-export-${Date.now()}.jsonl`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);

        this.showCopyFeedback(this.elements.exportBtn);
    },
};

// Setup copy button for individual messages (using event delegation)
document.addEventListener('click', async (e) => {
    if (e.target.classList.contains('timeline-copy-msg')) {
        const messageId = e.target.dataset.messageId;
        const message = Timeline.currentMessages.find(m => m.message_id === messageId);

        if (message) {
            const timestamp = new Date(message.timestamp);
            const text = `Time: ${timestamp.toISOString()}\nProvider: ${message.provider}\nProject: ${message.project || 'None'}\nRole: ${message.role}\n\n${message.content}`;

            try {
                await navigator.clipboard.writeText(text);
                Timeline.showCopyFeedback(e.target);
            } catch (error) {
                console.error('Copy failed:', error);
                alert('Failed to copy to clipboard');
            }
        }
    }
});
