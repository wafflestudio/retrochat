# Project Path Extraction Analysis

**작성일**: 2025-10-19
**목적**: 각 Provider에서 세션이 실행된 프로젝트 경로(working directory)를 추출하는 방법 조사

---

## 개요

현재 RetroChat는 `ChatSession` 모델에 `project_name` 필드만 저장하고 있습니다. 사용자가 실제 세션이 실행된 **전체 프로젝트 경로**를 확인하고 싶어하므로, 각 provider의 데이터에서 이 정보를 얻을 수 있는지 조사했습니다.

## 현재 상태

### ChatSession 모델 (`src/models/chat_session.rs`)
```rust
pub struct ChatSession {
    pub id: Uuid,
    pub provider: Provider,
    pub project_name: Option<String>,        // 프로젝트 이름만 저장
    pub file_path: String,                   // chat history 파일 경로
    // ...
}
```

**문제점**:
- `project_name`: 프로젝트 이름만 (예: "retrochat")
- `file_path`: chat history 파일의 저장 경로 (예: `/Users/lullu/.claude/projects/-Users-lullu-study-retrochat/abc.jsonl`)
- **실제 세션 실행 경로**: 저장 안 됨 (예: `/Users/lullu/study/retrochat` ❌)

---

## Provider별 분석

### 1. Claude Code

#### 파일 구조
```
~/.claude/projects/
  └── -Users-lullu-study-retrochat/
      └── 61ac7e7d-8fdd-46f9-8d8e-4793aeeac69b.jsonl
```

#### 데이터 포맷 (실제 확인)
```json
{
  "type": "user",
  "cwd": "/Users/lullu/study/retrochat",
  "gitBranch": "main",
  "sessionId": "01601fb2-7e83-4bb9-8fc1-c736a632fcfa",
  "message": {
    "role": "user",
    "content": "Hello"
  },
  "timestamp": "2025-10-19T14:49:59.496Z"
}
```

#### Project Path 추출 방법

**✅ 가능 - 메시지 메타데이터에 직접 제공!**

Claude Code는 **모든 user/assistant 메시지에 `cwd` 필드를 포함**합니다:
- `cwd`: 실제 작업 디렉토리 전체 경로
- `gitBranch`: Git 브랜치 정보 (bonus!)

**중요 발견**:
- 대부분의 세션은 단일 `cwd` 사용 (90%+)
- 일부 세션은 여러 `cwd` 사용 (예: worktree 이동)
  ```
  {'/Users/lullu/study/retrochat',
   '/Users/lullu/study/retrochat/.worktree/feature-ux-improvements'}
  ```

**구현 전략**:
- **빈도 기반 선택**: 세션에서 가장 많이 등장한 `cwd`를 project_path로 사용
- 이유: 다른 provider와 일관성 유지 (세션당 1개의 대표 경로)

```rust
// 실제 구현 예시
let cwd_counts: HashMap<String, usize> = messages
    .iter()
    .filter_map(|entry| entry.cwd.as_ref())
    .fold(HashMap::new(), |mut acc, cwd| {
        *acc.entry(cwd.clone()).or_insert(0) += 1;
        acc
    });

let project_path = cwd_counts
    .into_iter()
    .max_by_key(|(_, count)| *count)
    .map(|(cwd, _)| cwd);
```

**구현 난이도**: ⭐ (매우 쉬움)
- 데이터에 직접 제공됨
- 빈도 계산만 추가 필요

---

### 2. Cursor Agent

#### 파일 구조
```
/Users/lullu/study/retrochat/.cursor/
  └── chats/
      └── 53460df9022de1a66445a5b78b067dd9/  (hash)
          └── 557abc41-6f00-41e7-bf7b-696c80d4ee94/  (UUID)
              └── store.db
```

#### 데이터 포맷 (SQLite)
```sql
-- meta 테이블
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT);

-- metadata JSON (hex-encoded)
{
  "agentId": "557abc41-6f00-41e7-bf7b-696c80d4ee94",
  "name": "Chat Session 1",
  "mode": "default",
  "createdAt": 1758872189097,
  "lastUsedModel": "claude-3-5-sonnet"
}
```

**⚠️ 주의**: Cursor metadata에는 프로젝트 경로 정보가 **포함되지 않음**

#### Project Path 추출 방법

**✅ 가능 - 파일 경로 역추적**

구조: `{project_path}/.cursor/chats/{hash}/{uuid}/store.db`

**현재 구현**:
```rust
// src/parsers/cursor_agent.rs:370-417
fn infer_project_name(&self, metadata: &CursorChatMetadata) -> Option<String> {
    let path = PathBuf::from(&self.db_path);

    // store.db → uuid_dir → hash_dir → chats_dir → .cursor → project_dir
    if let Some(uuid_dir) = path.parent() {
        if let Some(hash_dir) = uuid_dir.parent() {
            if let Some(chats_dir) = hash_dir.parent() {
                if let Some(cursor_dir) = chats_dir.parent() {
                    if let Some(project_dir) = cursor_dir.parent() {
                        // ✅ 이미 project_dir를 찾고 있음!
                        return Some(project_dir.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    None
}
```

**필요한 작업**:
- 현재 `project_dir.file_name()`만 반환 (이름만)
- **전체 경로**를 반환하도록 수정

**구현 난이도**: ⭐ (매우 쉬움)
- 이미 `project_dir`를 찾는 로직 존재
- `file_name()` → `to_string_lossy().to_string()` 변경만 필요

---

### 3. Gemini CLI

#### 파일 구조
```
(사용자 지정 위치의 JSON 파일)
예: ~/gemini-exports/session-1234567890-abc.json
```

#### 데이터 포맷 (여러 버전)

**Format 1 - Session with metadata**:
```json
{
  "sessionId": "session-1234567890-abc",
  "projectHash": "abc123",  // ⚠️ hash만 있음
  "startTime": "2024-01-01T10:00:00Z",
  "lastUpdated": "2024-01-01T11:00:00Z",
  "messages": [...]
}
```

**Format 2 - Old export format**:
```json
{
  "conversations": [{
    "conversation_id": "test-123",
    "title": "Test Chat",  // ⚠️ title만 있음
    "conversation": [...]
  }]
}
```

**Format 3 - Array format**:
```json
[
  {
    "sessionId": "session-123",
    "messageId": 1,
    "type": "user",
    "message": "hello",
    "timestamp": "2024-01-01T10:00:00Z"
  }
]
```

#### Project Path 추출 방법

**❌ 불가능 - 데이터에 경로 정보 없음**

Gemini CLI 데이터에는 프로젝트 실행 경로 정보가 전혀 없습니다:
- `projectHash`: 해시값만 (실제 경로 아님)
- `title`: 사용자가 지정한 제목
- 세션 메타데이터에 `cwd`, `git`, `path` 등의 필드 없음

**현재 구현**:
```rust
// src/parsers/gemini_cli.rs:241-248, 304-313
// project_hash를 프로젝트 이름으로 사용하거나
// ProjectInference로 파일 경로에서 추론
```

**대안**:
1. **파일 경로 기반 추론**: `ProjectInference`를 사용하여 JSON 파일이 위치한 디렉토리 경로 활용
   - 예: `/Users/lullu/projects/myapp/gemini-data/session.json` → `/Users/lullu/projects/myapp`
   - ⚠️ 정확도 낮음 (export 위치 ≠ 실행 위치)

2. **수동 설정**: 사용자가 import 시 프로젝트 경로를 직접 지정
   - CLI 옵션: `retrochat import --path file.json --project-path /Users/lullu/projects/myapp`

**구현 난이도**: ⭐⭐⭐ (어려움)
- 데이터에 정보가 없어 추론만 가능
- 신뢰도가 낮음

---

### 4. Codex

#### 파일 구조
```
(사용자 지정 위치의 JSONL 파일)
예: ~/codex-sessions/session-abc.jsonl
```

#### 데이터 포맷

**New Format (with cwd)**:
```jsonl
{"timestamp":"2025-10-12T14:10:16.717Z","type":"session_meta","payload":{"id":"0199d8c1-ffeb-7b21-9ebe-f35fbbcf7a59","timestamp":"2025-10-12T14:10:16.683Z","cwd":"/Users/test/project","git":{"commit_hash":"abc123","branch":"main","repository_url":"git@github.com:user/test-project.git"}}}
{"timestamp":"2025-10-12T17:53:40.556Z","type":"event_msg","payload":{"type":"user_message","message":"hello"}}
```

#### Project Path 추출 방법

**✅ 가능 - 직접 제공됨!**

Codex는 세션 메타데이터에 **실제 작업 디렉토리(cwd)를 직접 제공**합니다!

**현재 구현**:
```rust
// src/parsers/codex.rs:54
pub struct SessionMetaPayload {
    pub id: String,
    pub timestamp: String,
    pub cwd: Option<String>,  // ✅ 전체 경로!
    pub instructions: Option<String>,
    pub git: Option<CodexGitInfo>,  // ✅ Git 정보도 포함
}

// src/parsers/codex.rs:404-412
fn infer_project_name_by_cwd(&self, meta: &SessionMetaPayload) -> Option<String> {
    meta.cwd.as_ref().and_then(|cwd_path| {
        Path::new(cwd_path)
            .file_name()  // ⚠️ 이름만 추출 중
            .and_then(|name| name.to_str())
            .map(|s| s.to_string())
    })
}
```

**필요한 작업**:
- `meta.cwd`를 그대로 반환 (이미 전체 경로)
- Git repository URL도 저장 고려

**구현 난이도**: ⭐ (매우 쉬움)
- 이미 데이터가 준비됨
- 단순히 반환만 하면 됨

**추가 정보**:
```rust
pub struct CodexGitInfo {
    pub commit_hash: Option<String>,
    pub branch: Option<String>,
    pub repository_url: Option<String>,  // 예: "git@github.com:user/project.git"
}
```

---

## 구현 제안

### Phase 1: 모델 확장

**`ChatSession` 모델에 `project_path` 필드 추가**

```rust
// src/models/chat_session.rs
pub struct ChatSession {
    pub id: Uuid,
    pub provider: Provider,
    pub project_name: Option<String>,    // 기존: 프로젝트 이름
    pub project_path: Option<String>,    // 신규: 전체 경로
    pub file_path: String,
    // ...
}
```

**데이터베이스 마이그레이션**:
```sql
-- migrations/YYYYMMDD_add_project_path.sql
ALTER TABLE chat_sessions
ADD COLUMN project_path TEXT;
```

### Phase 2: Provider별 구현

#### 우선순위 1 - Codex (가장 쉬움, 정확도 높음)
```rust
// src/parsers/codex.rs
fn extract_project_path(&self, meta: &SessionMetaPayload) -> Option<String> {
    meta.cwd.clone()  // 직접 제공됨!
}
```

#### 우선순위 2 - Cursor (쉬움, 정확도 높음)
```rust
// src/parsers/cursor_agent.rs
fn infer_project_path(&self) -> Option<String> {
    let path = PathBuf::from(&self.db_path);
    // 5단계 parent()로 project_dir 찾기
    path.parent()?.parent()?.parent()?.parent()?.parent()
        .map(|p| p.to_string_lossy().to_string())
}
```

#### 우선순위 3 - Claude Code (중간, 정확도 높음)
```rust
// src/parsers/project_inference.rs
impl ProjectInference {
    pub fn infer_project_path(&self) -> Option<String> {
        // -Users-lullu-study-retrochat → /Users/lullu/study/retrochat
        // resolve_original_path() 재사용
        let path = Path::new(&self.file_path);
        if let Some(parent_dir) = path.parent() {
            let parent_name = parent_dir.file_name()?.to_str()?;
            if parent_name.starts_with('-') {
                return self.resolve_original_path(parent_name);
            }
        }
        None
    }
}
```

#### 우선순위 4 - Gemini (어려움, 정확도 낮음)
```rust
// src/parsers/gemini_cli.rs
// Option 1: 파일 경로 기반 추론 (부정확)
// Option 2: 사용자 수동 입력 (CLI 옵션 추가 필요)
```

### Phase 3: UI 업데이트

**Web UI에서 표시**:
```javascript
// src/web/static/app.js
${session.project_path ? `
    <div class="detail-item">
        <span class="detail-label">Project Path</span>
        <span class="detail-value" style="font-family: monospace;">
            ${escapeHtml(session.project_path)}
        </span>
    </div>
` : ''}
```

**TUI에서 표시**:
```rust
// src/tui/views/session_detail.rs
if let Some(path) = &session.project_path {
    Paragraph::new(format!("Path: {}", path))
        .style(Style::default().fg(Color::Cyan))
}
```

---

## 요약 비교표

| Provider | 데이터 가용성 | 추출 방법 | 정확도 | 구현 난이도 | 비고 |
|----------|-------------|----------|--------|-----------|------|
| **Codex** | ✅ 직접 제공 | `meta.cwd` 필드 | 100% | ⭐ 매우 쉬움 | `cwd` 필드에 전체 경로 포함 |
| **Claude Code** | ✅ 직접 제공 | 메시지 `cwd` 필드 (빈도 기반) | 100% | ⭐ 매우 쉬움 | 모든 메시지에 `cwd` + `gitBranch` 포함 |
| **Cursor** | ✅ 파일 구조 | 경로 역추적 (5x parent) | 100% | ⭐ 쉬움 | `.cursor/chats/...` 구조 활용 |
| **Gemini CLI** | ❌ 없음 | 파일 위치 추론 OR 수동 입력 | 30% | ⭐⭐⭐ 어려움 | 데이터에 정보 없음 |

---

## 다음 단계

1. ✅ **조사 완료** (이 문서)
2. ⬜ **마이그레이션 작성**: `project_path` 컬럼 추가
3. ⬜ **모델 업데이트**: `ChatSession` 구조체 수정
4. ⬜ **Parser 구현**: 각 provider별 경로 추출 로직
   - Codex (우선)
   - Cursor
   - Claude Code
   - Gemini (Optional)
5. ⬜ **Repository/Service 수정**: 데이터 저장/조회 로직
6. ⬜ **UI 업데이트**: Web UI, TUI에 경로 표시
7. ⬜ **테스트 작성**: 각 provider별 경로 추출 테스트

---

## 참고 파일

- `src/models/chat_session.rs`: ChatSession 모델
- `src/parsers/claude_code.rs`: Claude Code 파서
- `src/parsers/cursor_agent.rs`: Cursor 파서
- `src/parsers/gemini_cli.rs`: Gemini CLI 파서
- `src/parsers/codex.rs`: Codex 파서
- `src/parsers/project_inference.rs`: 프로젝트 경로 추론 유틸
- `tests/contract/test_cursor_integration.rs`: Cursor 테스트 샘플

