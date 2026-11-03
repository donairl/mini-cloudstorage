#!/usr/bin/env bash
# Dev: backend on :3000, frontend on :5173. Ctrl+C stops both.
set -euo pipefail
cd "$(dirname "$0")"

[ -f .env ] || sed "s/^JWT_SECRET=.*/JWT_SECRET=$(openssl rand -hex 32)/" .env.example > .env
[ -d frontend/node_modules ] || (cd frontend && npm install)

trap 'kill 0' EXIT
# The project sits on exFAT, which corrupts rustc artifacts; build outside it.
(cd backend && CARGO_TARGET_DIR="$HOME/.cache/minidrive-target" cargo run) &
(cd frontend && npm run dev) &

echo "Open http://localhost:5173 once the backend has compiled."
wait
