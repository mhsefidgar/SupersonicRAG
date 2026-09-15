$ErrorActionPreference = 'Stop'
Write-Host 'Starting SupersonicRAG local stack...'
docker compose up --build -d
Write-Host 'Admin:  http://localhost:3000'
Write-Host 'API:    http://localhost:8080/health'
Write-Host 'Qdrant: http://localhost:6333/dashboard'
