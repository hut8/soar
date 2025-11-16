#!/bin/bash
#
# Monitor migration progress
#
# Usage: ./monitor-migration-progress.sh [log_dir]
#

LOG_DIR="${1:-$(ls -td /tmp/migration-logs-* 2>/dev/null | head -1)}"

if [ -z "$LOG_DIR" ] || [ ! -d "$LOG_DIR" ]; then
    echo "Error: Could not find migration logs directory"
    echo "Usage: $0 [log_dir]"
    exit 1
fi

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "Monitoring migration progress: $LOG_DIR"
echo "Press Ctrl+C to exit"
echo ""

while true; do
    clear
    echo -e "${GREEN}==> Migration Progress Monitor${NC}"
    echo "Logs: $LOG_DIR"
    echo "Time: $(date)"
    echo ""

    # Count completed partitions
    TOTAL_LOGS=$(ls -1 "$LOG_DIR"/fixes_*.log 2>/dev/null | wc -l)
    COMPLETED=$(grep -l "Completed fixes_p" "$LOG_DIR"/fixes_*.log 2>/dev/null | wc -l)
    IN_PROGRESS=$((TOTAL_LOGS - COMPLETED))

    echo "Fixes Partitions:"
    echo "  Total: $TOTAL_LOGS"
    echo "  Completed: $COMPLETED"
    echo "  In Progress: $IN_PROGRESS"
    echo ""

    # Show recently completed
    echo "Recently Completed (last 5):"
    grep "Completed fixes_p" "$LOG_DIR"/fixes_*.log 2>/dev/null | tail -5 | sed 's/^/  /'
    echo ""

    # Show currently processing
    echo "Currently Processing:"
    grep "Starting update for" "$LOG_DIR"/fixes_*.log 2>/dev/null | grep -v "Completed" | tail -10 | sed 's/^/  /'
    echo ""

    # Show any errors
    ERRORS=$(grep -c "ERROR\|FAILED" "$LOG_DIR"/*.log 2>/dev/null | grep -v ":0$" | wc -l)
    if [ "$ERRORS" -gt 0 ]; then
        echo -e "${YELLOW}Errors detected in $ERRORS log files${NC}"
        grep "ERROR\|FAILED" "$LOG_DIR"/*.log 2>/dev/null | head -5 | sed 's/^/  /'
    fi

    sleep 2
done
