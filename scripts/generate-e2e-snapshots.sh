#!/bin/bash
#
# Generate E2E Test Baseline Snapshots
#
# This script:
# 1. Ensures backend and frontend are built
# 2. Seeds test database with data
# 3. Starts backend server in background
# 4. Starts frontend preview server in background
# 5. Runs Playwright tests with --update-snapshots
# 6. Cleans up background processes
#
# Usage:
#   ./scripts/generate-e2e-snapshots.sh

set -e  # Exit on error

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  Generate E2E Test Baseline Snapshots${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Store PIDs for cleanup
BACKEND_PID=""
FRONTEND_PID=""

# Cleanup function
cleanup() {
    echo ""
    echo -e "${YELLOW}Cleaning up background processes...${NC}"

    if [ -n "$BACKEND_PID" ]; then
        echo "Stopping backend server (PID: $BACKEND_PID)"
        kill $BACKEND_PID 2>/dev/null || true
    fi

    if [ -n "$FRONTEND_PID" ]; then
        echo "Stopping frontend server (PID: $FRONTEND_PID)"
        kill $FRONTEND_PID 2>/dev/null || true
    fi

    # Also kill any processes on our ports
    lsof -ti:61225 | xargs kill -9 2>/dev/null || true
    lsof -ti:4173 | xargs kill -9 2>/dev/null || true

    echo -e "${GREEN}✓ Cleanup complete${NC}"
}

# Register cleanup on exit
trap cleanup EXIT INT TERM

# Step 1: Build backend
echo -e "${YELLOW}[1/6] Building backend (cargo build --release)...${NC}"
if [ ! -f "target/release/soar" ]; then
    cargo build --release || {
        echo -e "${RED}Failed to build backend${NC}"
        exit 1
    }
else
    echo -e "${GREEN}✓ Backend binary already exists${NC}"
fi
echo ""

# Step 2: Build frontend
echo -e "${YELLOW}[2/6] Building frontend (npm run build)...${NC}"
cd web
if [ ! -d "build" ]; then
    npm run build || {
        echo -e "${RED}Failed to build frontend${NC}"
        exit 1
    }
else
    echo -e "${GREEN}✓ Frontend build already exists${NC}"
fi
cd ..
echo ""

# Step 3: Seed test database (allow failure if already seeded)
echo -e "${YELLOW}[3/6] Seeding test database...${NC}"
TEST_USER_EMAIL=test@example.com \
TEST_USER_PASSWORD=testpassword123 \
SEED_COUNT=20 \
DATABASE_URL=postgres://postgres:postgres@localhost:5432/soar_test \
./target/release/soar seed-test-data 2>&1 || {
    echo -e "${YELLOW}Note: Seed data may already exist (this is OK)${NC}"
}
echo ""

# Step 4: Start backend server
echo -e "${YELLOW}[4/6] Starting backend server on port 61225...${NC}"
DATABASE_URL=postgres://postgres:postgres@localhost:5432/soar_test \
SOAR_ENV=production \
SOAR_BIND_HOST=0.0.0.0 \
SOAR_BIND_PORT=61225 \
./target/release/soar web > /tmp/soar-backend.log 2>&1 &
BACKEND_PID=$!

# Wait for backend to be ready
echo "Waiting for backend to start..."
for i in {1..30}; do
    if curl -s http://localhost:61225/health > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Backend server running (PID: $BACKEND_PID)${NC}"
        break
    fi
    if [ $i -eq 30 ]; then
        echo -e "${RED}Backend failed to start${NC}"
        cat /tmp/soar-backend.log
        exit 1
    fi
    sleep 1
done
echo ""

# Step 5: Start frontend preview server
echo -e "${YELLOW}[5/6] Starting frontend preview server on port 4173...${NC}"
cd web
BACKEND_URL=http://localhost:61225 \
npm run preview > /tmp/soar-frontend.log 2>&1 &
FRONTEND_PID=$!

# Wait for frontend to be ready
echo "Waiting for frontend to start..."
for i in {1..30}; do
    if curl -s http://localhost:4173 > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Frontend server running (PID: $FRONTEND_PID)${NC}"
        break
    fi
    if [ $i -eq 30 ]; then
        echo -e "${RED}Frontend failed to start${NC}"
        cat /tmp/soar-frontend.log
        exit 1
    fi
    sleep 1
done
echo ""

# Step 6: Run Playwright tests with --update-snapshots
echo -e "${YELLOW}[6/6] Running Playwright tests to generate snapshots...${NC}"
echo -e "${BLUE}This will take several minutes...${NC}"
echo ""

TEST_USER_EMAIL=test@example.com \
TEST_USER_PASSWORD=testpassword123 \
DATABASE_URL=postgres://postgres:postgres@localhost:5432/soar_test \
npx playwright test --update-snapshots --reporter=list || {
    echo -e "${YELLOW}Some tests may have failed, but snapshots should be generated${NC}"
}

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Snapshot generation complete!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "1. Review generated snapshots in web/e2e/**/*-snapshots/"
echo "2. Commit them: git add web/e2e/**/*-snapshots/"
echo "3. Push: git commit -m 'test: add baseline snapshots' && git push"
echo ""
