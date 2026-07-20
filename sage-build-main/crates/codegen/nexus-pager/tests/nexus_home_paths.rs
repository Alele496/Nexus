//! `NEXUS_HOME` override tests in an isolated binary so `nexus_home()`'s
//! process-wide `OnceLock` initializes from the overridden env var.

use std::path::PathBuf;

#[test]
fn nexus_home_override_path_helpers() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let nexus_home = tmp.path().to_path_buf();
    unsafe {
        std::env::set_var("NEXUS_HOME", &nexus_home);
    }

    assert_eq!(
        nexus_pager::util::pager_toml_path(),
        nexus_home.join("pager.toml")
    );
    assert_eq!(
        nexus_pager::util::display_nexus_home_prefix(),
        "$NEXUS_HOME"
    );
    assert_eq!(
        nexus_pager::util::display_user_nexus_path("config.toml"),
        "$NEXUS_HOME/config.toml"
    );

    let memory_path = nexus_home.join("memory/MEMORY.md");
    assert_eq!(
        nexus_pager::util::abbreviate_path(&memory_path.display().to_string()),
        "$NEXUS_HOME/memory/MEMORY.md"
    );

    assert!(nexus_pager::util::is_under_user_nexus_home(&memory_path));
    assert!(!nexus_pager::util::is_under_user_nexus_home(
        PathBuf::from("/tmp/other").as_path()
    ));
}
