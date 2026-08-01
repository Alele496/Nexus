// Per-test-case module for the `pty_e2e` integration test crate.
#[allow(unused_imports)]
use super::common::*;

/// `k` on the blocked welcome screen unlocks into a working session.
///
/// Regression for the api_key gate: with `preferred_method = "api_key"` and no
/// key configured, the pager blocks on the welcome screen ("press k to set
/// one"). Pressing `k` and entering a key must persist it to auth.json and
/// unlock straight into a usable session (the running agent's key provider
/// re-reads auth.json on its next call — no restart), then a real turn with
/// the just-entered key must reach the wire and render.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn welcome_api_key_k_unlocks_into_session() {
    let content = ContentController::start().await.expect("start content");
    content.set_response("MOCKRESPONSE from the unlocked api_key session.");

    // Mark the first-run wizard done and pin preferred_method=api_key in the
    // isolated NEXUS_HOME (faithful to a BYOK deployment that has completed
    // setup), then spawn WITHOUT an API key env var: no cached token, no env
    // key, no IdP — the welcome gate must block with the api_key error.
    let nexus_home = content.home().join(".nexus");
    std::fs::create_dir_all(&nexus_home).expect("create temp .nexus");
    std::fs::write(
        nexus_home.join("config.toml"),
        "[startup]\nwizard_completed = true\nusername = \"Developer\"\n\
         [auth]\npreferred_method = \"api_key\"\n",
    )
    .expect("write wizard-completed config");

    let binary = pager_binary().expect("resolve pager binary");
    let env = oauth_env_for_pager(&content);
    let env_refs: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let mut harness = PtyHarness::new(&binary, DEFAULT_ROWS, DEFAULT_COLS, &[], &env_refs)
        .expect("spawn pager without api key");
    // The full app queries the terminal (cursor position / device attributes)
    // at startup; answer them so it can render the first frame.
    harness.set_respond_to_queries(true);

    // 1. Gate: no API key is configured yet.
    harness
        .wait_for_text("no API key is configured", WELCOME_TIMEOUT)
        .expect("api_key gate on welcome screen");

    // 2. Press k: key-entry mode replaces the menu (collapses to Quit) with a
    //    masked input box.
    harness.inject_keys(b"k").expect("enter api key mode");
    harness
        .wait_for_text("Paste or type your API key", Duration::from_secs(5))
        .expect("api key entry hint");

    // 3. Type the key, then submit with a separate Enter (keeps the events
    //    unambiguous through the ConPTY line discipline).
    harness
        .inject_keys(b"sk-pty-unlock-key")
        .expect("type api key");
    harness.update(Duration::from_millis(300));
    harness.inject_keys(b"\r").expect("submit api key");

    // 4. Key entry closes and the pager transitions to the main session.
    harness
        .wait_for_text_absent("Paste or type your API key", Duration::from_secs(10))
        .expect("key entry closed after submit");
    harness
        .wait_for_text_absent("no API key is configured", Duration::from_secs(10))
        .expect("gate message gone after k flow");

    // 5. End-to-end: a turn with the just-entered key reaches the wire and the
    //    mock response renders.
    harness.inject_keys(b"go\r").expect("submit prompt");
    harness
        .wait_for_text(MOCK_RESPONSE_SENTINEL, Duration::from_secs(30))
        .expect("mock response after unlocking with api key");

    if harness.contains_text("panicked") {
        panic!("pager panicked\n{}", harness.screen_contents());
    }

    harness.quit().expect("clean quit");
}
