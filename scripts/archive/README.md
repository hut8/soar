# Archived Migration Scripts

## ⚠️ DO NOT USE THESE SCRIPTS

These scripts are kept for **historical reference only** and should **NOT** be used.

## Why These Scripts Are Archived

These scripts were created for the **December 18-21, 2025 incident** where DEFAULT partitions accumulated millions of rows due to connection exhaustion preventing partman maintenance from running.

**Problems with these scripts:**
1. **Hard-coded dates** (2025-12-18, 2025-12-19, 2025-12-20)
2. **Not reusable** for future incidents
3. **Lack comprehensive documentation**
4. **Replaced by generalized solution**

## What To Use Instead

**Use the generalized script:** `../fix-partitioned-table.sh`

```bash
# Fix any partitioned table with DEFAULT partition problems
../fix-partitioned-table.sh <database> <table_name> <partition_key> <timezone>

# Examples:
../fix-partitioned-table.sh soar fixes received_at '+01'
../fix-partitioned-table.sh soar raw_messages received_at '+01'
```

The new script:
- ✅ Automatically detects date ranges
- ✅ Works for any partitioned table
- ✅ Includes comprehensive documentation
- ✅ Explains historical context and root cause
- ✅ Reusable for future occurrences

## Contents

- `migrate-default-partition.sql` - Hard-coded migration for fixes table (Dec 18-20, 2025)
- `migrate-raw-messages-default-partition.sql` - Hard-coded migration for raw_messages table (Dec 18-21, 2025)

## Historical Context

See `../PARTITION_MIGRATION_GUIDE.md` for complete details about:
- What happened in December 2025
- Root cause analysis
- Prevention measures implemented
- How to handle future occurrences
