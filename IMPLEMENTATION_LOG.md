# Implementation Log: Fix No Models and Send Button Disabled

## 2026-02-21 – Model Discovery and Logging Plan

### Git Setup
- Added `upstream` remote to remote-frontend: `https://github.com/BloopAI/vibe-kanban.git`
- Merged upstream v0.1.18 (TanStack Router, review annotation fix, Electric fix)
- Resolved package.json conflict (kept local dev script with CCR env vars)
- **Note:** Create a GitHub fork and set `origin` to your fork for pushing. Run `git stash drop` to remove the stash if no longer needed.

### Model Discovery from API Keys
- **New:** `remote-frontend/crates/executors/src/executors/claude/model_discovery.rs`
  - `fetch_anthropic_models()` – GET https://api.anthropic.com/v1/models
  - `fetch_gemini_models()` – GET https://generativelanguage.googleapis.com/v1beta/models
  - `fetch_models_from_apis()` – parallel fetch, merge with fallback, dedupe by id
- **Integrated** into Claude `discover_options` stream: fetches models early, yields `update_models` and `models_loaded` patches
- **Env vars:** `DISCOVERY_ANTHROPIC_API_KEY`, `DISCOVERY_GEMINI_API_KEY` (or `GEMINI_API_KEY`). Skips `ANTHROPIC_API_KEY` when it is `ccr-local-dev-key`
- **package.json:** Passes `DISCOVERY_ANTHROPIC_API_KEY` and `DISCOVERY_GEMINI_API_KEY` ($GEMINI_API_KEY) to backend

### CCR Config Fixes
- **ccr-data/config.json:** Gemini `api_base_url` changed from `.../v1beta/models/` to `.../v1beta/`
- **ccr-data/config.json:** Added `gemini-2.5-flash`, `gemini-2.5-pro` to Gemini models

### Logging and Docs
- **REMOTE_BACKEND_FIX.md:** CCR logging section (log location, tail command, what to look for)
- **REMOTE_BACKEND_FIX.md:** Model discovery env vars
- **.env:** Comment for DISCOVERY_* vars

---

## 2025-02-21 – Plan Implementation Complete

### Changes Made

#### 1. Stream state initialization (Phase 4.1)
- **Files:** `remote-frontend/frontend/src/hooks/useJsonPatchWsStream.ts`, `vibe-kanban/frontend/src/hooks/useJsonPatchWsStream.ts`
- **Change:** Call `setData(initial)` after `dataRef.current = initial` so the UI has defined state before the first WebSocket patch (avoids undefined/config null during loading).

#### 2. Flat model list (Phase 4.3)
- **Files:** `remote-frontend/crates/executors/src/executors/claude.rs`, `vibe-kanban/crates/executors/src/executors/claude.rs`
- **Change:** Simplified `default_discovered_options()`:
  - `providers: vec![]` (removed provider accordion)
  - `provider_id: None` on all models
  - Model ids use CCR format: `anthropic,claude-3-7-sonnet-20250219`, `openrouter,google/gemini-2.5-pro-preview`, etc.
  - `default_model: "anthropic,claude-3-7-sonnet-20250219"`

#### 3. Diagnostic logging (removed in Phase 6.3)
- Phase 3 and Phase 5 diagnostic logging was added for debugging and then removed during cleanup.

### Verification Steps (Manual)

1. **Phase 0–2:** Backend builds without qa-mode; CCR running; WebSocket delivers models.
2. **Phase 4:** Models appear in the model selector dropdown (flat list).
3. **Phase 5:** Send button enables when repos, branches, message, and project are set.
4. **Phase 6:** End-to-end flow works; optionally test `Router.default` with Gemini/OpenRouter.

### CCR Config Reference

- Transformers: `gemini` and `openrouter` providers have `transformer: { "use": ["gemini"] }` and `transformer: { "use": ["openrouter"] }`.
- `Router.default`: Format `provider,model` (e.g. `"anthropic,claude-3-7-sonnet-20250219"`).
- Run `ccr restart` after config changes.
