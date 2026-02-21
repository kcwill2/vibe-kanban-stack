docker compose --env-file ../../.env.remote down

# 2. Delete the hidden volume cache (Safe - only affects the router's local storage)
docker volume prune -f
