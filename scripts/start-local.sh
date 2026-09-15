#!/usr/bin/env bash
set -euo pipefail

echo "Starting SupersonicRAG local stack..."
docker compose up --build -d
echo "Admin: http://localhost:3000"
echo "API:   http://localhost:8080/health"
echo "Qdrant:http://localhost:6333/dashboard"
