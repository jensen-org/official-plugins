#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

npx esbuild src/main.ts --bundle --format=esm --outfile=main.js

echo "built main.js"
echo "next: jensen publish $(pwd)"
