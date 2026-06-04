# Changelog

All notable changes to Glean will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Homebrew Cask distribution
- VitePress documentation site
- Demo GIF / screenshots

## [0.1.0] — 2026-06-04

### Added — Phase 0: Foundation
- Tauri v2 + Vue 3.5 + TypeScript project skeleton
- Pinia 3 + VueUse + Tailwind CSS v4 + Lucide icons
- SQLite (rusqlite + rusqlite_migration) + FTS5 full-text index
- `notify`-based file watcher (FSEvents on macOS)
- System tray icon + global shortcut (`⌘+⇧+Space`)
- GitHub Actions: CI + Release workflows

### Added — Phase 1: File Indexing
- `walkdir`-based recursive file scanner with parallel hashing
- Smart ignore rules (`.git` / `node_modules` / `.DS_Store` / >100MB / iCloud placeholders)
- xxhash3 for fast content hashing
- Index scheduler with priority queue and background throttling
- Real-time watcher for incremental updates
- Spotlight metadata via `mdls`

### Added — Phase 2: Semantic Search
- `fastembed`-based local embedding (BGE-small-zh-v1.5)
- sqlite-vec for vector storage
- Hybrid search: BM25 + vector KNN with RRF fusion
- Search palette (`⌘K`) with virtual scrolling
- jieba Chinese tokenization

### Added — Phase 3: Agent Capabilities
- LLM provider abstraction (`LLMProvider` trait)
- OpenAI-compatible provider (works with DeepSeek/Zhipu/Moonshot/Tongyi)
- Streaming chat with SSE
- BYOK configuration in Settings
- Chat panel with Markdown rendering + highlight.js + DOMPurify
- RAG pipeline: hybrid retrieval → augmented query → streaming response
- Conversation history persistence
- **Agent tool calling** (Phase 3 Week 10):
  - 5 built-in tools: `search_files`, `read_file`, `list_similar`, `move_file`, `tag_file`
  - OpenAI function-calling protocol with streaming tool_calls accumulation
  - Agent loop (max 5 iterations)
  - **Safety**: user confirmation required for destructive tools (move/tag)
  - **Undo stack**: `operations` table records reverse info; `undo_operation` command
  - Confirmation registry with per-conversation keying

### Added — Phase 4 Week 11: UX Polish
- **File preview panel**: PDF iframe, Markdown render, hljs code highlight, image preview
- **Tag system**: `TagBadge` component + `tags`/`file_tags` tables + 8 preset colors
- **Favorites**: `favorites` table + toggle + sidebar/file-row star buttons
- **Recently viewed**: `recently_viewed` table + auto-tracking + sidebar entry
- **Onboarding wizard**: 3-step first-run guide (welcome → folders → start)
- **Settings page**: LLM/ignore rules/theme/language/shortcuts/privacy/stats
- **i18n**: vue-i18n v9 with zh-CN / en locales (~15 domains)
- **Theme**: light/dark/system with `matchMedia` listener
- **Sidebar nav**: All Files / Recent / Favorites with active state
- **Context menu** in file list: open / reveal / copy path

### Fixed — Code review batch 1
- **Searcher deadlock**: split `search()` into outer lock + inner `search_on_conn(&Connection)` to avoid recursive `std::sync::Mutex` lock
- **MoveFile DB sync**: `UPDATE files.path` after rename; undo also syncs
- **ConfirmationRegistry namespace**: key changed to `(conversation_id, call_id)` to avoid cross-conversation conflicts
- **chat_stop blocks pending confirms**: `cancel_conversation()` broadcasts `false` to all pending receivers
- **JSON corruption**: tool error JSON built via `serde_json::json!` instead of hand-concatenation
- **UTF-8 chunk boundary**: SSE stream buffer changed to `Vec<u8>`, full-line `from_utf8` avoids splitting multi-byte chars
- **OpenAI stream accumulator**: `index` missing → skip chunk; empty `id`/`name` → don't overwrite; `[DONE]` → break inner loop

### Fixed — Code review batch 2 (frontend)
- `ensureListeners` uses single `listenersReady` flag + `listenersPromise` mutex to prevent duplicate registration
- `stopGenerate` clears `pendingConfirmations` and resets `loading`/`streaming` defensively
- `respondConfirmation` invokes first, mutates state only on success
- `newConversation` clears `pendingConfirmations`
- File row `:key` changed from `idx` to `tc.callId` for stable reactivity

### Infrastructure
- ESLint flat config with browser globals (`globals` package)
- VitePress documentation site (deployed to GitHub Pages)
- Issue templates (bug report / feature request)
- PR template + CONTRIBUTING.md
- Bilingual README (zh-CN / en)
- RELEASING.md with signing and updater key generation guide

### Known Limitations
- Unsigned build — users must run `xattr -dr com.apple.quarantine` on first launch
- macOS only (13.0+)
- No local LLM yet (planned: MLX)
- No image content understanding (planned: Phase 5)
- No mobile sync (planned: Phase 5 with CloudKit)

### Dependencies
- Tauri 2.x
- Vue 3.5
- Pinia 3
- Tailwind CSS v4
- rusqlite with bundled SQLite
- reqwest for HTTP
- fastembed for local embeddings
- vue-i18n v9
- VitePress v1.5

---

## Versioning

Glean follows Semantic Versioning:
- **MAJOR**: breaking changes (e.g., database schema migration required)
- **MINOR**: new features, backward-compatible
- **PATCH**: bug fixes, backward-compatible
