//! Filesystem locations for nexus config files and binaries.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static NEXUS_HOME: OnceLock<PathBuf> = OnceLock::new();

/// 重定向文件名：保存在 OS 默认 `~/.nexus/` 下，内容为用户选择的 Nexus 数据目录路径。
/// 解决无 bat 脚本直接双击 exe 时 `NEXUS_HOME` 环境变量无法跨进程持久化的问题。
const NEXUS_HOME_REDIRECT_FILENAME: &str = "nexus-home-path";

#[cfg(target_os = "macos")]
const CLAUDE_MANAGED_SETTINGS_PATH: &str =
    "/Library/Application Support/ClaudeCode/managed-settings.json";
#[cfg(target_os = "linux")]
const CLAUDE_MANAGED_SETTINGS_PATH: &str = "/etc/claude-code/managed-settings.json";

/// The default user nexus directory (`~/.nexus`, canonicalized) used when
/// `NEXUS_HOME` is unset. Exposed so callers (e.g. display helpers) can detect
/// whether [`nexus_home()`] is the default without duplicating the computation.
///
/// Uses [`dunce::canonicalize`] instead of [`std::fs::canonicalize`]: on
/// Windows, std returns a verbatim path (`\\?\C:\Users\...`) which external
/// tools choke on — e.g. `git clone` rejects `\\?\` destinations with
/// "Invalid argument", breaking marketplace cache clones under
/// `~/.nexus/marketplace-cache`. `dunce` strips the prefix whenever the path
/// is safely representable in legacy form; on non-Windows it is identical to
/// `std::fs::canonicalize`.
///
/// Keep the dunce canonicalization in sync with the hand-rolled duplicate in
/// `nexus_fast_worktree::db::resolve_nexus_home` (deliberately standalone crate).
pub fn default_nexus_home() -> PathBuf {
    #[allow(deprecated)]
    let home = std::env::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let dot_nexus = dunce::canonicalize(&home).unwrap_or(home).join(".nexus");

    // Check redirect file: written by first-run wizard when user picks a
    // custom data directory. Allows cross-process persistence without
    // requiring NEXUS_HOME env var to be set externally.
    if let Some(redirected) = read_nexus_home_redirect(&dot_nexus) {
        return redirected;
    }

    dot_nexus
}

/// Read the redirect file at `dot_nexus/nexus-home-path`. Returns the
/// redirected path if it exists, is absolute, and the directory exists.
fn read_nexus_home_redirect(dot_nexus: &Path) -> Option<PathBuf> {
    let redirect_file = dot_nexus.join(NEXUS_HOME_REDIRECT_FILENAME);
    let content = std::fs::read_to_string(&redirect_file).ok()?;
    let path = PathBuf::from(content.trim());
    if path.is_absolute() && path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Per-user config directory: `$NEXUS_HOME` or `~/.nexus`. Created if needed.
pub fn nexus_home() -> PathBuf {
    NEXUS_HOME
        .get_or_init(|| {
            let nexus_home = if let Ok(v) = std::env::var("NEXUS_HOME") {
                PathBuf::from(v)
            } else {
                default_nexus_home()
            };
            let _ = std::fs::create_dir_all(&nexus_home);
            nexus_home
        })
        .clone()
}

/// The user-global sage home, but only when one genuinely resolves: `Some` when
/// `$NEXUS_HOME` is set or a home directory is found, `None` otherwise. Unlike
/// [`nexus_home()`], this never falls back to a cwd-relative `.nexus`, so callers
/// that *scan* user-global sage resources (hooks, marketplace sources, ...) don't
/// mistake a project's `.nexus` tree for the user-global one when no home resolves.
pub fn user_nexus_home() -> Option<PathBuf> {
    #[allow(deprecated)]
    let resolvable = std::env::var_os("NEXUS_HOME").is_some() || std::env::home_dir().is_some();
    resolvable.then(nexus_home)
}

/// Canonical sage application path: `$NEXUS_HOME/bin/sage` (Unix) or `sage.exe` (Windows).
pub fn nexus_application() -> PathBuf {
    nexus_application_in(&nexus_home())
}

/// [`nexus_application`] under an explicit home instead of `$NEXUS_HOME`.
pub fn nexus_application_in(home: &std::path::Path) -> PathBuf {
    let name = if cfg!(windows) { "nexus.exe" } else { "nexus" };
    home.join("bin").join(name)
}

/// System-wide config directory: `/etc/sage/` on Unix, `None` on Windows.
pub fn system_config_dir() -> Option<PathBuf> {
    if cfg!(unix) {
        Some(PathBuf::from("/etc/sage"))
    } else {
        None
    }
}

/// System path for the managed-settings.json used for settings compat, if it exists.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn claude_managed_settings_path() -> Option<PathBuf> {
    let path = PathBuf::from(CLAUDE_MANAGED_SETTINGS_PATH);
    path.exists().then_some(path)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn claude_managed_settings_path() -> Option<PathBuf> {
    None
}

/// The platform path where managed-settings.json would live for settings
/// compat, whether or not it exists. `None` on unsupported platforms.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn claude_managed_settings_probe_path() -> Option<PathBuf> {
    Some(PathBuf::from(CLAUDE_MANAGED_SETTINGS_PATH))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn claude_managed_settings_probe_path() -> Option<PathBuf> {
    None
}

/// Max bytes for a single directory name component (macOS APFS, Linux ext4,
/// NTFS all enforce 255 bytes).
const MAX_DIRNAME_BYTES: usize = 255;

/// Encode a CWD string into a filesystem-safe directory name component.
///
/// Short CWDs (URL-encoded form <= 255 bytes) use URL-encoding for backward
/// compatibility and human readability on disk.
///
/// Long CWDs (> 255 bytes encoded) use a compact `{slug}-{blake3_hex16}`
/// form that is always <= 57 bytes. Callers must write a `.cwd` metadata
/// file via [`ensure_sessions_cwd_dir`] so the original CWD can be
/// recovered by [`decode_cwd_from_dirname`].
pub fn encode_cwd_dirname(cwd: &str) -> String {
    let url_encoded = urlencoding::encode(cwd);
    if url_encoded.len() <= MAX_DIRNAME_BYTES {
        return url_encoded.into_owned();
    }
    let hash = blake3::hash(cwd.as_bytes());
    let hash16 = &hash.to_hex()[..16];
    let leaf = std::path::Path::new(cwd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workspace");
    let slug = slugify(leaf, 40);
    let slug = if slug.is_empty() { "workspace" } else { &slug };
    format!("{slug}-{hash16}")
}

/// Every on-disk session cwd-dir key that may hold sessions for `cwd`.
///
/// The same Windows directory is reachable under several path-separator
/// styles (`F:\a\b`, `F:/a\b`, `F:/a/b`), and session dirs were historically
/// keyed by the raw URL-encoding of whichever cwd string created them. Lookups
/// must therefore probe every form; writers (`ensure_sessions_cwd_dir`) keep
/// using [`encode_cwd_dirname`] alone, so new sessions stay under one key.
///
/// On non-Windows platforms this is just the single [`encode_cwd_dirname`]
/// result. Results are deduplicated and in priority order.
pub fn encode_cwd_dirname_candidates(cwd: &str) -> Vec<String> {
    let mut keys: Vec<String> = Vec::with_capacity(3);
    let mut push = |k: String| {
        if !keys.contains(&k) {
            keys.push(k);
        }
    };
    push(encode_cwd_dirname(cwd));
    #[cfg(windows)]
    {
        // `F:/a\b`: the form produced by `PathBuf::join` on a redirect-file
        // home like `F:/Sage-home` — the common key for worktree sessions.
        let bs = cwd.replace('/', "\\");
        let bytes = bs.as_bytes();
        if bytes.len() >= 2 && bytes[1] == b':' {
            // `F:/Sage-home\...` — drive-letter forward slash, remaining
            // separators backslash. Drop the separator right after the drive
            // letter (already included as `/`).
            let rest = bs[2..].strip_prefix(['\\', '/']).unwrap_or(&bs[2..]);
            push(encode_cwd_dirname(&format!("{}/{}", &bs[..2], rest)));
        }
        // All-forward-slash form.
        push(encode_cwd_dirname(&cwd.replace('\\', "/")));
    }
    keys
}

/// Recover the original CWD from a sessions CWD directory.
///
/// Tries URL-decoding the directory name first (works for short/legacy dirs).
/// Falls back to reading a `.cwd` metadata file inside the directory (written
/// by [`ensure_sessions_cwd_dir`] for hash-based dirs).
pub fn decode_cwd_from_dirname(dir: &std::path::Path) -> Option<String> {
    let name = dir.file_name()?.to_str()?;
    if let Ok(decoded) = urlencoding::decode(name) {
        let s = decoded.into_owned();
        // URL-decoded absolute CWDs always start with `/` (Unix) or a drive
        // letter (Windows).  The slug-hash form never does, so this
        // distinguishes the two encodings unambiguously.
        if s.starts_with('/') || (cfg!(windows) && s.chars().nth(1) == Some(':')) {
            return Some(s);
        }
    }
    std::fs::read_to_string(dir.join(".cwd"))
        .ok()
        .map(|s| s.trim().to_string())
}

/// Build the CWD-level session directory path:
/// `nexus_home()/sessions/{encode_cwd_dirname(cwd)}`.
///
/// Does **not** create the directory on disk — use [`ensure_sessions_cwd_dir`]
/// when the directory must exist.
pub fn sessions_cwd_dir(cwd: &str) -> PathBuf {
    nexus_home().join("sessions").join(encode_cwd_dirname(cwd))
}

/// Create the CWD-level session directory and write a `.cwd` metadata file
/// when hash-based encoding is used (long paths).
///
/// For short paths the `.cwd` file is not written because the directory name
/// itself is reversible via URL-decoding.
pub fn ensure_sessions_cwd_dir(cwd: &str) -> std::io::Result<PathBuf> {
    let encoded_name = encode_cwd_dirname(cwd);
    let dir = nexus_home().join("sessions").join(&encoded_name);
    std::fs::create_dir_all(&dir)?;
    // Hash-based encoding is in use when the dirname differs from the
    // plain URL-encoded form.  Write a `.cwd` file so decode can recover
    // the original path.  O_CREAT|O_EXCL via create_new avoids TOCTOU
    // races with parallel session starts.
    if encoded_name != urlencoding::encode(cwd).as_ref() {
        let cwd_file = dir.join(".cwd");
        match std::fs::File::create_new(&cwd_file) {
            Ok(mut f) => {
                std::io::Write::write_all(&mut f, cwd.as_bytes())?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Ok(dir)
}

/// Generate a URL-safe slug from a string.
///
/// Lowercases, replaces non-alphanumeric chars with `-`, collapses
/// consecutive dashes, and truncates to `max_len` characters.
fn slugify(input: &str, max_len: usize) -> String {
    let mut result = String::with_capacity(input.len());
    let mut prev_dash = false;
    for c in input.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c);
            prev_dash = false;
        } else if !prev_dash {
            result.push('-');
            prev_dash = true;
        }
    }
    let trimmed = result.trim_matches('-');
    trimmed.chars().take(max_len).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Realistic CWDs that trigger the bug (URL-encoded > 255 bytes).
    const LONG_CWDS: &[&str] = &[
        "/Users/dev/Documents/開発プロジェクト/機能追加/テスト環境/ソースコード/main-branch",
        "/Users/user/Library/Mobile Documents/com~apple~CloudDocs/项目文件/深层嵌套目录/更深层次的/工作区域/project",
        "/Users/user/Library/CloudStorage/OneDrive-대한민국회사/프로젝트/개발환경/소스코드/백엔드/서비스/my-app",
        "/Users/user/Documents/工作文件夹/二零二六年项目/子目录一/子目录二/子目录三/源代码/code",
    ];

    #[test]
    fn long_cwd_uses_hash_fallback_within_name_max() {
        let long_cwd = format!("/Users/test/{}", "中".repeat(30));
        let encoded = encode_cwd_dirname(&long_cwd);
        assert!(encoded.len() <= MAX_DIRNAME_BYTES);
        assert!(!encoded.starts_with("%2F"));
    }

    #[test]
    fn different_long_paths_produce_different_hashes() {
        let a = format!("/Users/test/{}", "中".repeat(30));
        let b = format!("/Users/test/{}", "日".repeat(30));
        assert_ne!(encode_cwd_dirname(&a), encode_cwd_dirname(&b));
    }

    #[test]
    fn decode_reads_cwd_file_for_hash_dirs() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("some-slug-abcdef0123456789");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".cwd"), "/original/long/path").unwrap();
        assert_eq!(
            decode_cwd_from_dirname(&dir),
            Some("/original/long/path".to_string())
        );
    }

    #[test]
    fn decode_returns_none_without_cwd_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("some-slug-abcdef0123456789");
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(decode_cwd_from_dirname(&dir), None);
    }

    #[test]
    fn cwd_file_write_is_idempotent_via_excl() {
        let tmp = TempDir::new().unwrap();
        let long_cwd = format!("/Users/test/{}", "中".repeat(30));
        let dir = tmp.path().join(encode_cwd_dirname(&long_cwd));
        std::fs::create_dir_all(&dir).unwrap();
        let cwd_file = dir.join(".cwd");
        std::fs::write(&cwd_file, &long_cwd).unwrap();
        match std::fs::File::create_new(&cwd_file) {
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            other => panic!("expected AlreadyExists, got: {other:?}"),
        }
        assert_eq!(std::fs::read_to_string(&cwd_file).unwrap(), long_cwd);
    }

    #[test]
    fn url_encoded_long_cwd_fails_on_real_filesystem() {
        let tmp = TempDir::new().unwrap();
        let url_encoded = urlencoding::encode(LONG_CWDS[0]).into_owned();
        let result = std::fs::create_dir_all(tmp.path().join(&url_encoded));
        assert!(result.is_err());
    }

    #[test]
    fn full_roundtrip_on_real_filesystem_for_long_cwds() {
        let tmp = TempDir::new().unwrap();
        for cwd in LONG_CWDS {
            let encoded = encode_cwd_dirname(cwd);
            let dir = tmp.path().join(&encoded);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(".cwd"), cwd).unwrap();
            assert_eq!(decode_cwd_from_dirname(&dir).as_deref(), Some(*cwd));
        }
    }

    #[test]
    fn short_cwds_use_url_encoding_and_roundtrip_on_real_filesystem() {
        let tmp = TempDir::new().unwrap();
        for cwd in [
            "/Users/foo/project",
            "/tmp",
            "/Users/user/Documents/project-名前",
        ] {
            let encoded = encode_cwd_dirname(cwd);
            assert_eq!(encoded, urlencoding::encode(cwd).into_owned());
            let dir = tmp.path().join(&encoded);
            std::fs::create_dir_all(&dir).unwrap();
            assert_eq!(decode_cwd_from_dirname(&dir).as_deref(), Some(cwd));
        }
    }

    #[test]
    fn default_nexus_home_has_no_verbatim_prefix() {
        // On Windows, std::fs::canonicalize returns `\\?\C:\...` verbatim
        // paths that external tools (notably `git clone`) reject. The dunce
        // canonicalization must yield a plain path. No-op assertion on Unix.
        let home = default_nexus_home();
        assert!(!home.to_string_lossy().starts_with(r"\\?\"));
        // The `.nexus` suffix only holds for the default location. A redirect
        // file (`~/.nexus/nexus-home-path`, written by the first-run wizard)
        // points the home at a user-chosen data directory instead.
        #[allow(deprecated)]
        let dot_nexus = std::env::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".nexus");
        if read_nexus_home_redirect(&dot_nexus).is_none() {
            assert!(home.ends_with(".nexus"));
        }
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello World!", 40), "hello-world");
    }

    #[test]
    fn slugify_cjk_produces_empty() {
        assert_eq!(slugify("深层目录", 40), "");
    }

    #[test]
    fn slugify_truncates() {
        assert_eq!(slugify(&"a".repeat(100), 10).len(), 10);
    }

    #[test]
    fn candidates_first_is_primary_and_deduplicated() {
        let keys = encode_cwd_dirname_candidates("/home/user/project");
        assert_eq!(keys.len(), 1, "non-Windows paths yield exactly one key");
        assert_eq!(keys[0], encode_cwd_dirname("/home/user/project"));
    }

    #[test]
    fn candidates_always_contain_the_primary_key() {
        for cwd in [
            "F:\\Sage-home\\worktrees\\git-agent-sys\\nexus-agent",
            "F:/Sage-home\\worktrees\\git-agent-sys\\nexus-agent",
            "E:\\Git仓库\\Agent-SYS",
        ] {
            let keys = encode_cwd_dirname_candidates(cwd);
            assert!(keys.contains(&encode_cwd_dirname(cwd)), "missing primary for {cwd}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_backslash_cwd_includes_legacy_mixed_drive_key() {
        // A session written with cwd `F:/Sage-home\...` (drive-letter forward
        // slash, backslashes elsewhere) must be findable from the canonical
        // `F:\Sage-home\...` form used by resume scans.
        let backslash = "F:\\Sage-home\\worktrees\\git-agent-sys\\nexus-agent";
        let mixed = "F:/Sage-home\\worktrees\\git-agent-sys\\nexus-agent";
        let keys = encode_cwd_dirname_candidates(backslash);
        assert!(
            keys.contains(&encode_cwd_dirname(mixed)),
            "backslash lookup must probe the legacy mixed-separator key"
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_mixed_cwd_still_matches_its_own_primary() {
        let mixed = "F:/Sage-home\\worktrees\\git-agent-sys\\nexus-agent";
        let keys = encode_cwd_dirname_candidates(mixed);
        assert!(keys.contains(&encode_cwd_dirname(mixed)));
    }
}
