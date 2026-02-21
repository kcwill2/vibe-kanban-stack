# Vibe Kanban Stack — Docker Setup (Self-Hosted)

This document describes the unified Docker Compose configuration that runs the Vibe Kanban **server** (main app API), the self-hosted **remote** backend (OAuth and cloud sync), and Claude Code Router (CCR) for AI model routing.

---

## 1. Components and Services

| Component | Type | Purpose |
|-----------|------|---------|
| **vk-remote** | Container (server binary) | Main application backend. Serves `/api/*` (projects, tasks, task-attempts, scratch, sessions, WebSockets). Uses PostgreSQL for app data, CCR for Anthropic AI calls, and the remote backend for OAuth handoff and sync. |
| **remote-server** | Container (remote binary) | Self-hosted “cloud” API. Serves `/v1/*` (OAuth, organizations, projects, issues, workspaces). Handles GitHub OAuth flow and stores tokens; used by the server for login and remote sync. |
| **postgres** | Container | PostgreSQL 15 for the **server** binary (app DB). |
| **remote-db** | Container | PostgreSQL 16 with `wal_level=logical` for the **remote** binary and ElectricSQL. |
| **ccr** | Container | [Claude Code Router](https://github.com/musistudio/claude-code-router). Proxies Anthropic API requests; allows routing to different models and providers. |
| **electric** | Container | ElectricSQL for real-time sync in the remote backend. Depends on remote-db and remote-server. |
| **azurite** | Container | Azure Blob Storage emulator for remote backend (e.g. issue attachments). |
| **azurite-init** | One-off job | Creates blob container and CORS rules for Azurite. |
| **Vite dev server** | Host (npm) | Serves the frontend at `http://127.0.0.1:5173` and proxies `/api` and `/v1` to the backends. Not in Docker. |

---

## 2. System Topology

Plain-text diagram (displays in all viewers):

```
  [Browser]  ---- :5173 ---->  [Vite dev server :5173]
                                    |
                    +---------------+---------------+
                    |                               |
              proxy /api                      proxy /v1
                    |                               |
                    v                               v
            [Host :3000]                     [Host :3001]
                    |                               |
                    v                               v
  +-----------------+-----------------+   [remote-server :8081]
  |  vk-remote (server) :3000          |         |
  |  Serves /api/*                    |         |
  +-----------------+-----------------+         |
            |         |           |             |
            v         v           v             v
      [postgres]   [ccr :3456]   [remote]   [remote-db] [azurite] [electric]
       :5432       (AI proxy)    :8081         |            |          |
            ^         ^           ^            +------------+----------+
            |         |           |                     |
            +---------+-----------+---------------------+
                    (VK_SHARED_API_BASE, ANTHROPIC_BASE_URL)
```

- **Browser** opens the app at `http://127.0.0.1:5173` (Vite).
- **Vite** proxies `/api` to host :3000 (server) and `/v1` to host :3001 (remote).
- **Server** (vk-remote) calls remote at `http://remote-server:8081`, CCR at `http://ccr:3456`, and postgres.
- **Remote** uses remote-db, azurite, and electric. Browser is redirected to :3001 for OAuth callback.

Mermaid version (for GitHub and Mermaid-capable previews; Cursor plan viewer supports it, regular project Markdown preview may not):

```mermaid
graph TB
    subgraph host [Host]
        Browser["Browser :5173"]
        Vite["Vite Dev Server :5173"]
    end

    subgraph server_stack [Server stack]
        Server["vk-remote server :3000<br/>Serves /api/*"]
        PG["postgres :5432"]
        CCR["ccr :3456"]
    end

    subgraph remote_stack [Remote stack]
        Remote["remote-server :8081<br/>Serves /v1/* OAuth"]
        RDB["remote-db :5432"]
        Electric["electric :3000"]
        Azurite["azurite :10000"]
    end

    Browser -->|":5173"| Vite
    Vite -->|"proxy /api -> :3000"| Server
    Vite -->|"proxy /v1 -> :3001"| Remote
    Server -->|"VK_SHARED_API_BASE"| Remote
    Server -->|"ANTHROPIC_BASE_URL"| CCR
    Server --> PG
    Remote --> RDB
    Remote --> Azurite
    Remote --> Electric
    Electric --> RDB
    Browser -.->|"OAuth redirect"| Remote
```

---

## 3. Docker Compose (`docker-compose.yml`)

The stack is defined in the repo root `docker-compose.yml`. All services share the default network so they resolve each other by service name.

### 3.1 Services overview

| Service | Image / build | Port mapping | Depends on |
|---------|----------------|--------------|------------|
| **postgres** | `postgres:15-alpine` | `5433:5432` | — |
| **vk-remote** | Build `./vibe-kanban` Dockerfile | `3000:3000` | postgres, ccr |
| **ccr** | `musistudio/claude-code-router:latest` | `${CCR_PORT:-3456}:3456` | — |
| **remote-db** | `postgres:16-alpine` | `5435:5432` | — |
| **azurite** | `mcr.microsoft.com/azure-storage/azurite:latest` | `10000:10000` | — |
| **azurite-init** | `mcr.microsoft.com/azure-cli:latest` | — | azurite (healthy) |
| **electric** | `electricsql/electric:1.3.3` | — | remote-db, remote-server |
| **remote-server** | Build `./vibe-kanban` crates/remote/Dockerfile | `3001:8081` | remote-db, azurite-init |

### 3.2 Build definitions

- **vk-remote:** `context: ./vibe-kanban`, `dockerfile: Dockerfile` — builds the **server** binary and runs it on port 3000 inside the container.
- **remote-server:** `context: ./vibe-kanban`, `dockerfile: crates/remote/Dockerfile` — builds the **remote** binary; optional build args: `FEATURES`, `POSTHOG_API_KEY`, `POSTHOG_API_ENDPOINT`, `SENTRY_DSN_REMOTE`.

**Build hang after “Image … Built”:** On some setups (especially Windows), `docker compose build` can hang after both images are built because of Docker Buildx provenance attestation. The images are already built; you can press **Ctrl+C** and run `docker compose up -d`. To avoid the hang on future builds, disable attestations before building (PowerShell: `$env:BUILDX_NO_DEFAULT_ATTESTATIONS="1"; docker compose build --no-cache`).

### 3.3 Volumes

| Volume | Used by | Purpose |
|--------|---------|---------|
| **remote-db-data** | remote-db | PostgreSQL data for the remote backend. |
| **electric-data** | electric | ElectricSQL persistent data. |
| **azurite-data** | azurite | Blob storage data. |
| **ccr-data** (bind) | ccr | Repo root `./ccr-data` mounted at `/root/.claude-code-router` for CCR config. |

### 3.4 Key environment variables per service

- **vk-remote:** `DATABASE_URL`, `VIBEKANBAN_REMOTE_JWT_SECRET`, `VK_ALLOWED_ORIGINS`, `ANTHROPIC_BASE_URL`, `VK_SHARED_API_BASE`.
- **ccr:** `PORT`, `ANTHROPIC_API_KEY`.
- **remote-server:** `VIBEKANBAN_REMOTE_JWT_SECRET`, `GITHUB_*`, `SERVER_DATABASE_URL`, `VIBEKANBAN_AUTH_PROVIDERS`, `SERVER_LISTEN_ADDR`, `ELECTRIC_URL`, `SERVER_PUBLIC_BASE_URL`, `ELECTRIC_ROLE_PASSWORD`, Azurite vars, `VK_ALLOWED_ORIGINS`.
- **electric:** `DATABASE_URL` (uses `ELECTRIC_ROLE_PASSWORD` from env), `PG_PROXY_PORT`, `LOGICAL_PUBLISHER_HOST`, and other Electric env vars.

All `${VAR}` values are taken from the root `.env` when you run `docker compose`.

### 3.5 Startup order

1. postgres, remote-db, azurite, ccr start (no dependency).
2. azurite-init runs after azurite is healthy, then exits.
3. vk-remote starts after postgres and ccr.
4. remote-server starts after remote-db is healthy and azurite-init has completed.
5. electric starts after remote-db and remote-server are healthy.

---

## 4. Ports and Addresses

### 4.1 Host-exposed ports (host:container)

| Host address | Container | Service | Used for |
|--------------|-----------|---------|----------|
| `127.0.0.1:3000` | vk-remote:3000 | server | Main API and WebSockets. Vite proxies `/api` here. Frontend and server health/API calls. |
| `127.0.0.1:3001` | remote-server:8081 | remote | Remote API and OAuth. Browser hits this for `/v1/oauth/github/start` and `/v1/oauth/github/callback`. |
| `127.0.0.1:3456` | ccr:3456 | ccr | Claude Code Router (optional direct use; server uses it via Docker network). |
| `127.0.0.1:5173` | — | Vite (host) | Frontend dev server; not in Docker. |
| `127.0.0.1:5433` | postgres:5432 | postgres | Server DB (optional host access). |
| `127.0.0.1:5435` | remote-db:5432 | remote-db | Remote DB (optional host access). |
| `127.0.0.1:10000` | azurite:10000 | azurite | Blob storage (optional host access). |

### 4.2 Internal (container-only, no host port)

| Address | Service | Used for |
|---------|---------|----------|
| `postgres:5432` | postgres | Server `DATABASE_URL`. |
| `remote-db:5432` | remote-db | Remote `SERVER_DATABASE_URL` and Electric `DATABASE_URL`. |
| `remote-server:8081` | remote-server | Server calls remote via `VK_SHARED_API_BASE`. |
| `ccr:3456` | ccr | Server AI calls via `ANTHROPIC_BASE_URL`. |
| `electric:3000` | electric | Remote backend real-time sync. |
| `azurite:10000` | azurite | Remote backend blob storage. |

---

## 5. Configuration Elements Required

### 5.1 Root `.env` (repo root)

Used by `docker compose` and by services that reference `${VAR}`.

| Variable | Purpose | Example |
|----------|---------|---------|
| `VIBEKANBAN_REMOTE_JWT_SECRET` | Shared JWT secret for server and remote (tokens). | Base64 from `openssl rand -base64 48` |
| `ANTHROPIC_API_KEY` | Anthropic API key for CCR. | `sk-ant-...` |
| `VK_ALLOWED_ORIGINS` | CORS origins for server and remote (frontend origin). | `http://127.0.0.1:5173,http://localhost:5173` |
| `VK_SHARED_API_BASE` | URL the **server** uses to call the remote (must be reachable from vk-remote container). | `http://remote-server:8081` |
| `GITHUB_CLIENT_ID` | GitHub OAuth App Client ID. | From GitHub OAuth App |
| `GITHUB_CLIENT_SECRET` | GitHub OAuth App Client Secret. | From GitHub OAuth App |
| `GITHUB_OAUTH_CLIENT_ID` | Same as `GITHUB_CLIENT_ID` (remote reads both). | Same value |
| `GITHUB_OAUTH_CLIENT_SECRET` | Same as `GITHUB_CLIENT_SECRET`. | Same value |
| `VIBEKANBAN_AUTH_PROVIDERS` | Auth providers for remote (e.g. `github`). | `github` |
| `SERVER_DATABASE_URL` | PostgreSQL URL for the **remote** service (remote-db). | `postgres://remote:remote@remote-db:5432/remote` |
| `ELECTRIC_ROLE_PASSWORD` | Password for ElectricSQL role on remote-db. | Chosen secret |
| `SERVER_PUBLIC_BASE_URL` | Public URL of the remote (for OAuth redirect_uri). Must be what the browser uses. | `http://localhost:3001` |
| `PUBLIC_BASE_URL` | Optional; some configs use it for the same purpose. | `http://localhost:3001` |
| `CCR_PORT` | Host port for CCR (optional). | `3456` (default) |

### 5.2 GitHub OAuth App

- **Authorization callback URL:** `http://localhost:3001/v1/oauth/github/callback`
- Optionally add: `http://127.0.0.1:3001/v1/oauth/github/callback` if the browser uses 127.0.0.1.
- Client ID and Secret must match `.env` (`GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`).

See [GITHUB_OAUTH_SETUP.md](GITHUB_OAUTH_SETUP.md) for steps.

### 5.3 CCR config (bind-mounted)

- **Volume:** `./ccr-data` → `/root/.claude-code-router` in the CCR container.
- **Content:** `config.json` and other CCR config. CCR uses `ANTHROPIC_API_KEY` from the container env.

**Additional providers (OpenRouter, Gemini, LM Studio):**

- **OpenRouter** — Set `OPENROUTER_API_KEY` in `.env` (get a key at [openrouter.ai](https://openrouter.ai)). The config references it via `$OPENROUTER_API_KEY`. Models are named like `google/gemini-2.5-pro-preview`, `anthropic/claude-3.5-sonnet`.
- **Gemini** — Set `GEMINI_API_KEY` in `.env` (from [Google AI Studio](https://aistudio.google.com/apikey)). Config uses `$GEMINI_API_KEY`. Models include `gemini-2.0-flash`, `gemini-1.5-pro`.
- **LM Studio (local)** — Runs on the host. In LM Studio: load a model, open the Developer tab, and start the server (default port 1234). CCR reaches it via `host.docker.internal:1234`. Edit `ccr-data/config.json` and set the `lmstudio` provider’s `models` array to the exact name shown in LM Studio (e.g. `["codellama/Qwen2.5-Coder-7B-Instruct-GGUF"]`). On Linux, `extra_hosts` in docker-compose already maps `host.docker.internal` to the host.

To use a different default model, change `Router.default` in `config.json` to `provider,model` (e.g. `openrouter,anthropic/claude-3.5-sonnet` or `gemini,gemini-2.0-flash`). You can also use CCR’s `/model` command in Claude Code to switch at runtime.

**Router.default and 400 "Missing model in request body":** `Router.default` must be set in `config.json` (e.g. `"anthropic,claude-3-7-sonnet-20250219"`). If the client does not send a model in the request body, CCR is designed to use this default. If you still see 400, the client (Claude Code / ccr code) may not be including the model in the request—a known issue ([CCR #774](https://github.com/musistudio/claude-code-router/issues/774)). Workarounds: run `ccr ui` once to configure the client, or ensure you select a model in the Vibe Kanban UI (Anthropic, OpenRouter, or Gemini) so the executor passes the model in CCR format (`provider,model`).

### 5.4 Vite proxy (frontend)

In `frontend/vite.config.ts` the dev server must proxy:

- `/api` → `http://127.0.0.1:3000` with `ws: true`
- `/v1` → `http://127.0.0.1:3000` (or to 3001 if you want the frontend to talk to remote directly for some calls; for the described setup, server talks to remote and frontend only needs `/api` to the server)

For the current setup the frontend only needs to reach the **server** on 3000; the server calls the remote internally. If you add frontend calls to `/v1`, point that proxy to `http://127.0.0.1:3001`.

---

## 6. Quick reference

- **Start stack:** `docker compose up -d` (from repo root).
- **Start frontend:** `pnpm run dev` (from repo root).
- **App URL:** http://127.0.0.1:5173
- **Server API:** http://127.0.0.1:3000/api/*
- **Remote API (OAuth, etc.):** http://127.0.0.1:3001/v1/*
- **Health:** `curl http://127.0.0.1:3000/api/health` and `curl http://127.0.0.1:3001/v1/health`
- **CCR model switch:** Change `Router.default` in `ccr-data/config.json` to `provider,model`, or use the `/model` command in Claude Code.
