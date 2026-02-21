# 1. Take the stack down
docker compose --env-file ../../.env.remote down

# 2. Delete the hidden volume cache (Safe - only affects the router's local storage)
docker volume prune -f

# 3. Bring it back up
docker compose --env-file ../../.env.remote up -d

# 4. Check the file content again
docker exec -it remote-claude-code-router-1 cat /root/.claude-code-router/config.json