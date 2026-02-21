# 1. Kill all Docker UI and Daemon processes
taskkill /F /IM "Docker Desktop.exe" /T
taskkill /F /IM "dockerd.exe" /T
taskkill /F /IM "wslservice.exe" /T

# 2. Force-shutdown the entire WSL2 subsystem
wsl --shutdown

# 3. Reset the Windows Socket API (The "Connection Closed" Fix)
netsh winsock reset