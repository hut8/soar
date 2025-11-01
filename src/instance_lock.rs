use anyhow::{Context, Result};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Instance lock to prevent multiple instances of the application from running simultaneously
pub struct InstanceLock {
    lock_file: File,
    lock_path: PathBuf,
}

impl InstanceLock {
    /// Create a new instance lock with the given name
    /// The lock file will be created in the system's runtime directory
    pub fn new(name: &str) -> Result<Self> {
        let lock_path = Self::get_lock_path(name)?;

        // Ensure the parent directory exists
        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create lock directory")?;
        }

        // Try to open the lock file with exclusive access
        let lock_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&lock_path)
            .context("Failed to open lock file")?;

        // Try to acquire an exclusive lock (non-blocking)
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = lock_file.as_raw_fd();
            let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
            if result != 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::WouldBlock {
                    anyhow::bail!(
                        "Another instance is already running. Lock file: {}",
                        lock_path.display()
                    );
                } else {
                    return Err(err).context("Failed to acquire lock");
                }
            }
        }

        #[cfg(windows)]
        {
            // Windows doesn't have flock, so we rely on CREATE_NEW failing if the file exists
            // This isn't perfect, but it's better than nothing
            if lock_path.exists() {
                anyhow::bail!(
                    "Another instance may be running. Lock file exists: {}",
                    lock_path.display()
                );
            }
        }

        // Write PID to the lock file
        let pid = std::process::id();
        let mut lock_file_clone = lock_file
            .try_clone()
            .context("Failed to clone file handle")?;
        writeln!(lock_file_clone, "{}", pid).context("Failed to write PID to lock file")?;

        info!("Acquired instance lock at {}", lock_path.display());
        debug!("Process ID: {}", pid);

        Ok(Self {
            lock_file,
            lock_path,
        })
    }

    /// Get the path for the lock file
    fn get_lock_path(name: &str) -> Result<PathBuf> {
        // Use XDG runtime directory on Linux, fallback to temp directory
        let runtime_dir = if let Ok(xdg_runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            PathBuf::from(xdg_runtime_dir)
        } else {
            std::env::temp_dir()
        };

        Ok(runtime_dir.join(format!("{}.lock", name)))
    }

    /// Get the lock file path
    pub fn path(&self) -> &Path {
        &self.lock_path
    }

    /// Check if a lock with the given name is currently held by another process
    /// Returns true if the lock is held, false otherwise
    pub fn is_locked(name: &str) -> Result<bool> {
        let lock_path = Self::get_lock_path(name)?;

        if !lock_path.exists() {
            return Ok(false);
        }

        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;

            // Try to open the lock file
            let lock_file = match OpenOptions::new().read(true).open(&lock_path) {
                Ok(file) => file,
                Err(_) => return Ok(false), // File doesn't exist or can't be opened
            };

            // Try to acquire a non-blocking exclusive lock
            let fd = lock_file.as_raw_fd();
            let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };

            if result != 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::WouldBlock {
                    // Lock is held by another process
                    return Ok(true);
                }
            }

            // We acquired the lock, release it immediately
            unsafe {
                libc::flock(fd, libc::LOCK_UN);
            }

            Ok(false)
        }

        #[cfg(not(unix))]
        {
            // On non-Unix systems, just check if the file exists
            Ok(lock_path.exists())
        }
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        // Release the lock
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = self.lock_file.as_raw_fd();
            unsafe {
                libc::flock(fd, libc::LOCK_UN);
            }
        }

        // Remove the lock file
        if let Err(e) = std::fs::remove_file(&self.lock_path) {
            eprintln!("Failed to remove lock file: {}", e);
        } else {
            debug!("Released instance lock at {}", self.lock_path.display());
        }
    }
}
