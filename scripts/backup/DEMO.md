# restore-backup Script Demo

This document demonstrates the usage of the `restore-backup` script.

## Script Overview

The `restore-backup` script is a comprehensive backup restoration tool that provides:
- Interactive Terminal User Interface (TUI) for backup selection
- Command-line modes for automation
- Integration with existing restore infrastructure
- Comprehensive error handling and confirmations

## Usage Examples

### 1. Interactive TUI Mode (Default)

```bash
./scripts/restore-backup
```

This displays an interactive menu:

```
╔═══════════════════════════════════════════════════════════════════════╗
║          SOAR Backup Restoration - Select a Backup                    ║
╚═══════════════════════════════════════════════════════════════════════╝
                ↑/↓: Navigate | Enter: Select | q/Esc: Cancel
────────────────────────────────────────────────────────────────────────

★ 2025-01-10 (2 days ago, 85.3 GB) | DB: 420.5 GB
  2025-01-03 (9 days ago, 84.8 GB) | DB: 415.2 GB  
  2024-12-27 (16 days ago, 84.1 GB) | DB: 410.8 GB
  2024-12-20 (23 days ago, 83.5 GB) | DB: 405.3 GB
► 2024-12-13 (30 days ago, 82.9 GB) | DB: 400.1 GB

                    Showing 1-5 of 5
```

**Navigation:**
- **↑/↓** or **k/j**: Move selection
- **Enter**: Select backup
- **q** or **Esc**: Cancel

**Features:**
- Latest backup marked with ★ (green)
- Current selection highlighted (inverted colors)
- Shows backup age, size, and database size
- Scroll indicator for large lists

### 2. List Available Backups

```bash
./scripts/restore-backup --list
```

**Output:**
```
Enumerating backups from cloud storage...
Found 5 backup(s)

Available backups:
----------------------------------------------------------------------
★ LATEST 2025-01-10 (2 days ago, 85.3 GB)
           Timestamp: 2025-01-10_00-15-23
           Source: prod-soar-01.example.com
         2025-01-03 (9 days ago, 84.8 GB)
           Timestamp: 2025-01-03_00-12-45
           Source: prod-soar-01.example.com
         2024-12-27 (16 days ago, 84.1 GB)
           Timestamp: 2024-12-27_00-11-03
           Source: prod-soar-01.example.com
----------------------------------------------------------------------
```

### 3. Restore Latest Backup

```bash
./scripts/restore-backup --latest
```

**Interactive Confirmation:**
```
Latest backup: 2025-01-10 (2 days ago, 85.3 GB)

══════════════════════════════════════════════════════════════════
⚠️  WARNING: DESTRUCTIVE OPERATION ⚠️
══════════════════════════════════════════════════════════════════

You are about to restore from backup:
  Date: 2025-01-10
  Age: 2 days ago
  Size: 85.3 GB

Backup details:
  Timestamp: 2025-01-10_00-15-23
  Source: prod-soar-01.example.com
  Database size: 420.5 GB

This will:
  1. Stop PostgreSQL
  2. DESTROY the current database
  3. Restore from backup
  4. Replay WAL logs to the latest available point

Estimated downtime: 2-4 hours

══════════════════════════════════════════════════════════════════

Type 'yes' to proceed (anything else to cancel): yes

══════════════════════════════════════════════════════════════════
Starting restore from backup: 2025-01-10
══════════════════════════════════════════════════════════════════

Executing: /path/to/restore --base-backup 2025-01-10 --latest --yes

Restore logs will be shown below...
══════════════════════════════════════════════════════════════════

[2025-01-12 10:00:00 UTC] [INFO] Restore: Starting database restore
[2025-01-12 10:00:01 UTC] [INFO] Restore: Using base backup: 2025-01-10
[2025-01-12 10:00:02 UTC] [INFO] Restore: Downloading base backup...
... (restore process continues)
```

### 4. Restore Specific Backup by Date

```bash
./scripts/restore-backup --date 2024-12-27
```

Prompts for confirmation, then restores the specified backup.

### 5. Using Custom Configuration

```bash
./scripts/restore-backup --config /custom/path/backup-env --list
```

## Configuration

The script reads from `/etc/soar/backup-env`:

```bash
# Required settings
BACKUP_RCLONE_REMOTE=wasabi
BACKUP_RCLONE_BUCKET=soar-backup-prod
RCLONE_CONFIG=/etc/soar/rclone.conf

# Optional prefix path within bucket
BACKUP_RCLONE_PATH=
```

## Error Handling

### Missing rclone
```
Error: rclone command not found

Please install rclone:
  curl https://rclone.org/install.sh | sudo bash
```

### Missing Configuration
```
Error: Configuration file not found: /etc/soar/backup-env

Please ensure /etc/soar/backup-env is properly configured.
```

### No Backups Found
```
No backups found!
```

### Invalid Date Format
```
Error: Backup not found for date: 2024-13-45

Available backups:
  2025-01-10
  2025-01-03
  2024-12-27
```

## Exit Codes

- **0**: Success
- **1**: Failure (error during execution)
- **2**: User cancelled (no changes made)

## Integration with Existing Infrastructure

The script integrates with:
1. **rclone**: For cloud storage access (Wasabi S3)
2. **backup/restore**: Existing low-level restore script
3. **/etc/soar/backup-env**: Shared configuration
4. **systemd**: Can be used in systemd units for automation

## Features

### TUI Features
- ✅ Curses-based interactive interface
- ✅ Keyboard navigation (arrow keys, vim keys)
- ✅ Visual highlighting and colors
- ✅ Latest backup indicator
- ✅ Scroll support for many backups
- ✅ Responsive to terminal size

### Data Display
- ✅ Backup date
- ✅ Backup age (human-readable)
- ✅ Backup size
- ✅ Database size from metadata
- ✅ Source hostname
- ✅ Backup timestamp

### Safety Features
- ✅ Explicit confirmation required
- ✅ Clear warning messages
- ✅ Shows what will be destroyed
- ✅ Allows cancellation at any point
- ✅ Non-destructive list mode

### Automation Features
- ✅ Command-line modes (--list, --latest, --date)
- ✅ Exit codes for scripting
- ✅ Can skip confirmation with careful usage
- ✅ Integrates with existing restore workflow

## Technical Details

- **Language**: Python 3.6+
- **Dependencies**: Standard library only (curses, subprocess, json, argparse)
- **Lines of Code**: ~600 lines
- **Test Coverage**: 7 unit tests, all passing
- **No .sh Extension**: Follows requirement for script naming

## Testing

Run the unit tests:

```bash
python3 scripts/backup/test_restore_backup.py
```

**Expected output:**
```
test_help_option ... ok
test_invalid_date_format ... ok
test_missing_config_file ... ok
test_missing_rclone_error ... ok
test_script_is_executable ... ok
test_shebang_is_correct ... ok
test_syntax_is_valid ... ok

----------------------------------------------------------------------
Ran 7 tests in 0.200s

OK
```
