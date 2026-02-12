#!/bin/sh
set -e

echo "Running database migrations..."
cd /app/migration
./migration up

echo "Starting application..."
cd /app
exec ./crypto-pocket-butler-backend
