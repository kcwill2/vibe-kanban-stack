# Remote Backend Connectivity Fix

## Problem

When running `npm run dev` from remote-frontend, you see:

```
Remote call failed, retrying: network error: error sending request for url (http://localhost:3001/v1/tokens/refresh)
```

**Cause**: The local server expects the remote backend at `http://localhost:3001` (set by `VK_SHARED_API_BASE` in `backend:dev:watch`), but nothing is listening on port 3001.

## Solution

Start the remote backend on port 3001 **without** starting vk-remote (which would conflict with the local server on port 3000).

### Step 1: Stop any conflicting containers

If you previously ran the vibe-kanban remote stack or root stack, stop it to free ports 5435, 10000, 3000, 3001:

```powershell
# From repo root
docker compose down

# Stop vibe-kanban remote stack (if used)
cd crates/remote
docker compose down
cd ../..
```

### Step 2: Start remote backend + CCR (no vk-remote)

Start only the services needed for local dev: remote-server on 3001, CCR on 3456, and their dependencies. **Skip vk-remote** so the local server can use port 3000.

```powershell
# From repo root
docker compose up -d remote-db azurite azurite-init electric remote-server ccr
```

This starts:
- **remote-server** on `localhost:3001` (OAuth, tokens, /v1 API)
- **ccr** on `localhost:3456` (AI routing)
- remote-db, azurite, electric (dependencies)

### Step 3: Run local dev

```powershell
# From repo root
pnpm run dev
```

- Local server: `http://localhost:3000`
- Remote backend: `http://localhost:3001` ✓
- CCR: `http://localhost:3456` ✓
- Frontend: `http://localhost:5173`

## Port Summary

| Service        | Port | Used by                    |
|----------------|------|----------------------------|
| Local server   | 3000 | npm run dev (backend)      |
| Remote backend | 3001 | Docker remote-server       |
| CCR            | 3456 | Docker ccr                 |
| Vite           | 5173 | npm run dev (frontend)     |
| remote-db      | 5435 | Docker                     |
| azurite        | 10000| Docker                     |

## Alternative: Use full Docker stack (no local server)

If you prefer to run everything in Docker:

```powershell
docker compose up -d
```

Then run only the frontend (no local server):

```powershell
cd frontend
npx cross-env VITE_VK_SHARED_API_BASE=http://localhost:3001 pnpm run dev -- --port 5173
```

The frontend will proxy to vk-remote on 3000, which talks to remote-server on 3001. This requires `VK_SHARED_API_BASE` to be set for vk-remote (in .env: `VK_SHARED_API_BASE=http://remote-server:8081` for Docker internal networking).

## CCR Logging (Debugging Model Errors)

CCR logs to `ccr-data/logs/` with filenames like `ccr-YYYYMMDDHHMMSS.log`. With `LOG: true` and `LOG_LEVEL: debug` in `ccr-data/config.json`, logs include:

- Incoming requests (URL, method)
- Request bodies (`data.model`, `data.messages`, etc.)
- Response status codes (`res.statusCode`)
- Response times

**To tail logs while debugging:**

```powershell
Get-Content ccr-data/logs/ccr-*.log -Tail 50 -Wait
```

**What to look for:** `statusCode: 404` or `statusCode: 401` indicates model or auth issues. The `data.model` field shows which model was requested.

## Model Discovery from API Keys

The model selector fetches available models from Anthropic and Gemini APIs when `DISCOVERY_ANTHROPIC_API_KEY` and/or `DISCOVERY_GEMINI_API_KEY` are set. (These are used instead of `ANTHROPIC_API_KEY` when it is overridden for CCR.) Add to `.env`:

```
DISCOVERY_ANTHROPIC_API_KEY=sk-ant-...
DISCOVERY_GEMINI_API_KEY=AIza...
```

If unset, the model list falls back to hardcoded defaults.
