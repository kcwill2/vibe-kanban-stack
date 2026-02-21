# GitHub OAuth setup (self-hosted remote)

For the self-hosted remote backend to complete sign-in, your GitHub OAuth App must use this callback URL:

- **Authorization callback URL:** `http://localhost:3001/v1/oauth/github/callback`
- Optional (if you use 127.0.0.1 in the browser): `http://127.0.0.1:3001/v1/oauth/github/callback`

**Where to set it:** GitHub → Settings → Developer settings → OAuth Apps → your app → Authorization callback URL.

Client ID and Secret must match the values in the root `.env` (`GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`).

## Next steps (after setting the callback URL)

1. Start the frontend: `pnpm run dev` (from repo root)
2. Open http://127.0.0.1:5173 and sign in with GitHub.
3. You should be redirected to GitHub, then to `http://localhost:3001/v1/oauth/github/callback`, then back to the app with "Signed in with github...". If you see "Authentication Failed", double-check the callback URL in GitHub and that the stack is running (`docker compose ps`).
