#!/usr/bin/env bash
# Remove retrochat database and related files

set -e

echo "Removing retrochat database and related files..."
rm -f ~/.retrochat/retrochat.db ~/.retrochat/retrochat.db-wal ~/.retrochat/retrochat.db-shm

echo "Database files removed:"
echo "  - ~/.retrochat/retrochat.db"
echo "  - ~/.retrochat/retrochat.db-wal"
echo "  - ~/.retrochat/retrochat.db-shm"
