//! Global shared mailbox store.
//!
//! The mailbox is a *cross-process* channel: any nexus process may
//! `send_message` to a session owned by another process (even one running
//! in a different terminal). Unlike other tool state — which lives in the
//! per-session `resources_state.json` — the mailbox therefore persists to a
//! single file under the Nexus home (`<home>/mailbox.json`) that all
//! processes share.
//!
//! Concurrency: every read-modify-write runs under an exclusive advisory
//! lock on a sidecar lock file (`<home>/mailbox.json.lock`, via `fs2`), so
//! two processes can never clobber each other's updates — the lock
//! serialises the whole read-modify-write, and the data file is replaced
//! atomically (`MoveFileExW` on Windows, `rename` on Unix, both with
//! durability flushes) so a reader always observes a complete snapshot.

use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use fs2::FileExt;

use super::types::MailboxState;

/// Filename of the shared mailbox store under the Nexus home directory.
pub const MAILBOX_STORE_FILENAME: &str = "mailbox.json";

/// A file-backed, cross-process mailbox store. Cheap to clone.
#[derive(Debug, Clone)]
pub struct MailboxStore {
    path: PathBuf,
    lock_path: PathBuf,
}

impl MailboxStore {
    /// Store rooted at the Nexus home — the shared location every process
    /// reads and writes.
    pub fn at_nexus_home() -> Self {
        let home = crate::util::nexus_home::nexus_home();
        Self::at_path(home.join(MAILBOX_STORE_FILENAME))
    }

    /// Store at an explicit data path (used by tests).
    pub fn at_path(data: impl Into<PathBuf>) -> Self {
        let data = data.into();
        let lock_path = data.with_extension("json.lock");
        Self { path: data, lock_path }
    }

    /// Load the current mailbox state. Returns an empty state when the file
    /// is missing; a corrupt file is logged and treated as empty rather
    /// than panicking.
    pub fn load(&self) -> MailboxState {
        match self.with_lock(|state| Ok(state.clone())) {
            Ok(state) => state,
            Err(error) => {
                tracing::warn!(
                    error = ?error,
                    path = ?self.path,
                    "failed to load mailbox store; using empty mailbox",
                );
                MailboxState::default()
            }
        }
    }

    /// Run `f` against the current state under an exclusive cross-process
    /// lock, then atomically persist the mutated state. `f`'s return value
    /// is returned to the caller. Persistence failures propagate as `Err`.
    pub fn update<R>(&self, f: impl FnOnce(&mut MailboxState) -> R) -> io::Result<R> {
        self.with_lock(|state| {
            let mut state = state.clone();
            let result = f(&mut state);
            Self::write_atomic(&self.path, &state)?;
            Ok(result)
        })
    }

    /// Run `f` while holding the exclusive cross-process lock.
    fn with_lock<R>(
        &self,
        f: impl FnOnce(&MailboxState) -> io::Result<R>,
    ) -> io::Result<R> {
        let lock_file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&self.lock_path)?;
        lock_file.lock_exclusive()?;
        let result = f(&Self::read(&self.path));
        // Unlock even if `f` errored. A failed unlock is only surfaced
        // when `f` itself succeeded (the original error takes precedence).
        let unlock_result = lock_file.unlock();
        result.and_then(|r| unlock_result.map(|_| r))
    }

    /// Read and parse the store file; missing or corrupt files yield an
    /// empty state (caller holds the lock). A corrupt file is quarantined
    /// (renamed aside with a timestamp) rather than silently overwritten
    /// by the next `update`, so recoverable data is never destroyed.
    fn read(path: &Path) -> MailboxState {
        match std::fs::File::open(path) {
            Ok(mut file) => {
                let mut text = String::new();
                if file.read_to_string(&mut text).is_ok()
                    && let Ok(state) = serde_json::from_str(&text)
                {
                    return state;
                }
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let backup = path.with_extension(format!("json.corrupt-{ts}"));
                tracing::warn!(
                    path = ?path,
                    ?backup,
                    "mailbox store unreadable or corrupt; quarantining and starting empty",
                );
                let _ = std::fs::rename(path, &backup);
            }
            Err(_) => {
                // Missing file is the normal first-run case.
            }
        }
        MailboxState::default()
    }

    /// Atomically replace `path` with `state`: write a temp file, sync it,
    /// then durably move it into place (see [`replace_durable`]). A failed
    /// write cleans up the temp file.
    fn write_atomic(path: &Path, state: &MailboxState) -> io::Result<()> {
        let json = serde_json::to_vec_pretty(state)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let tmp = path.with_extension("json.tmp");
        let result = (|| {
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&tmp)?;
            file.write_all(&json)?;
            file.sync_all()?;
            drop(file);
            replace_durable(path, &tmp)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        result
    }
}

/// Durably move `tmp` over `path`.
///
/// Windows cannot rename over an existing file, so we use
/// `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)` — an
/// atomic replacement with no crash window (a plain remove-then-rename
/// would lose the whole mailbox if the process died between the two
/// syscalls). Unix uses `rename` (already atomic) plus a parent-directory
/// fsync so the rename survives a power loss.
#[cfg(windows)]
fn replace_durable(path: &Path, tmp: &Path) -> io::Result<()> {
    use windows::Win32::Storage::FileSystem::MOVE_FILE_FLAGS;
    use windows::Win32::Storage::FileSystem::MoveFileExW;
    use windows::core::PCWSTR;

    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    }
    let from = windows_extended_path(tmp)?;
    let to = windows_extended_path(path)?;
    unsafe {
        MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            // MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH
            MOVE_FILE_FLAGS(1 | 8),
        )
    }
    .map_err(io::Error::other)
}

#[cfg(not(windows))]
fn replace_durable(path: &Path, tmp: &Path) -> io::Result<()> {
    std::fs::rename(tmp, path)?;
    if let Some(parent) = path.parent() {
        // Best-effort durability flush: a directory fsync failure must not
        // turn a *successful* rename into a reported error (that would
        // make the sender retry and duplicate the message). Same
        // convention as `persistence.rs` and the dashboard's
        // `atomic_write`.
        let _ = OpenOptions::new()
            .read(true)
            .open(parent)
            .and_then(|dir| dir.sync_all());
    }
    Ok(())
}

/// Windows: absolute path with the `\\?\` extended-length prefix, so long
/// or Unicode paths (e.g. under a non-ASCII user profile) work with the
/// Win32 move API.
#[cfg(windows)]
fn windows_extended_path(path: &Path) -> io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;
    let path = std::path::absolute(path)?;
    let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if wide.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path contains NUL",
        ));
    }
    let unc = wide.starts_with(&[92, 92]);
    let mut result = if unc { r"\\?\UNC\" } else { r"\\?\" }
        .encode_utf16()
        .collect::<Vec<_>>();
    if unc {
        wide.drain(..2);
    }
    result.extend(wide);
    result.push(0);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::implementations::nexus_build::mailbox::types::{
        AgentAddress, MailboxMessage, MessageStatus,
    };

    /// Unique-per-test scratch directory. Cargo runs tests in parallel, so
    /// sharing one file across tests would let them clobber each other;
    /// each test gets its own subdirectory keyed by a global counter.
    fn test_dir() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "nexus-mailbox-test-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    fn test_store() -> MailboxStore {
        MailboxStore::at_path(test_dir().join("mailbox.json"))
    }

    fn message(to: &str) -> MailboxMessage {
        MailboxMessage::new(
            AgentAddress::new("sender-session"),
            AgentAddress::new(to),
            "subject".to_string(),
            "body".to_string(),
            "normal".to_string(),
        )
    }

    #[test]
    fn missing_file_loads_empty() {
        let store = test_store();
        let state = store.load();
        assert!(state.messages.is_empty());
        assert!(state.labels.is_empty());
    }

    #[test]
    fn write_then_load_roundtrips() {
        let store = test_store();
        store
            .update(|s| s.insert(message("recipient-a")))
            .expect("update should persist");
        let state = store.load();
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].to.session_id, "recipient-a");
    }

    /// Two independent stores over the same file model two nexus
    /// processes: a message written by one must be visible to the other.
    #[test]
    fn cross_process_visibility() {
        let store_a = test_store();
        let store_b = store_a.clone();
        store_a
            .update(|s| s.insert(message("recipient-b")))
            .expect("process A send should persist");
        let state = store_b.load();
        let inbox = state.inbox_for("recipient-b");
        assert_eq!(inbox.len(), 1, "process B must see A's message");
    }

    /// A freshly constructed store (same path) still sees earlier writes —
    /// models process restart.
    #[test]
    fn survives_process_restart() {
        let path = test_dir().join("mailbox.json");
        {
            let store = MailboxStore::at_path(&path);
            store
                .update(|s| s.insert(message("restart-recipient")))
                .expect("first process write");
        }
        // New store over the same file = a new process starting up.
        let store = MailboxStore::at_path(&path);
        let state = store.load();
        let inbox = state.inbox_for("restart-recipient");
        assert_eq!(inbox.len(), 1, "message must survive process restart");
    }

    /// Marking a message read in one store is observed by the other.
    #[test]
    fn mark_read_is_shared() {
        let store_a = test_store();
        let store_b = store_a.clone();
        let id = store_a
            .update(|s| {
                let m = message("reader");
                let id = m.id.clone();
                s.insert(m);
                id
            })
            .expect("send");
        store_b
            .update(|s| {
                s.find_mut(&id).expect("message present").mark_read();
            })
            .expect("mark read");
        let state = store_a.load();
        assert_eq!(state.messages[0].status, MessageStatus::Read);
    }
}
