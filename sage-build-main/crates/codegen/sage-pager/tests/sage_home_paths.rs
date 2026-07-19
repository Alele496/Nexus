//! `SAGE_HOME` override tests in an isolated binary so `sage_home()`'s
//! process-wide `OnceLock` initializes from the overridden env var.

use std::path::PathBuf;

#[test]
fn sage_home_override_path_helpers() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let sage_home = tmp.path().to_path_buf();
    unsafe {
        std::env::set_var("SAGE_HOME", &sage_home);
    }

    assert_eq!(
        sage_pager::util::pager_toml_path(),
        sage_home.join("pager.toml")
    );
    assert_eq!(
        sage_pager::util::display_sage_home_prefix(),
        "$SAGE_HOME"
    );
    assert_eq!(
        sage_pager::util::display_user_sage_path("config.toml"),
        "$SAGE_HOME/config.toml"
    );

    let memory_path = sage_home.join("memory/MEMORY.md");
    assert_eq!(
        sage_pager::util::abbreviate_path(&memory_path.display().to_string()),
        "$SAGE_HOME/memory/MEMORY.md"
    );

    assert!(sage_pager::util::is_under_user_sage_home(&memory_path));
    assert!(!sage_pager::util::is_under_user_sage_home(
        PathBuf::from("/tmp/other").as_path()
    ));
}
