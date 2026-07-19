//! Core telemetry tracking — local session metrics only (no external calls).
//!
//! All xAI telemetry (Mixpanel, product events, Sentry) has been removed.
//! Precedence: env > config > remote config > default.

use std::sync::{Arc, Mutex, OnceLock};

use chrono::{Local, SecondsFormat};
use serde_json::json;

use crate::config::{TelemetryConfig, TelemetryMode, deployment_id_from_key};
use crate::http::OriginClientInfo;
use crate::session_ctx::EmitterOrigin;

/// Event property map shared by all telemetry modules.
pub type Metadata = serde_json::Map<String, serde_json::Value>;

/// Derive the analytics `event_value` from the full wire `event_name` by stripping
/// whichever [`EmitterOrigin`] prefix it carries (`sage-shell-` /
/// `sage-workspace-`). Unprefixed names pass through unchanged. Kept in
/// lockstep with [`EmitterOrigin::event_prefix`] via [`EmitterOrigin::ALL`],
/// so shell events keep their historical stripped value and workspace events
/// collapse to the same bare suffix.
fn event_value(event_name: &str) -> &str {
    for origin in EmitterOrigin::ALL {
        if let Some(suffix) = event_name.strip_prefix(origin.event_prefix()) {
            return suffix;
        }
    }
    event_name
}

#[derive(Clone)]
pub struct TelemetryClient {
    mode: TelemetryMode,
    user_id: Option<String>,
    team_id: Option<String>,
    deployment_id: Option<String>,
    shell_version: String,
    client_type: Option<String>,
    client_version: Option<String>,
    subscription_tier: Option<String>,
}

impl std::fmt::Debug for TelemetryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelemetryClient")
            .field("mode", &self.mode)
            .finish()
    }
}

impl TelemetryClient {
    pub fn from_config(
        config: TelemetryConfig,
        mode: TelemetryMode,
        user_id: Option<String>,
        team_id: Option<String>,
        deployment_key: Option<String>,
        origin_client: Option<OriginClientInfo>,
        shell_version: String,
        subscription_tier: Option<String>,
        _http_client: reqwest::Client,
    ) -> Self {
        let deployment_id = deployment_key
            .filter(|s| !s.is_empty())
            .map(|k| deployment_id_from_key(&k));
        let (client_type, client_version) = match origin_client {
            Some(o) => (Some(o.product), o.version),
            None => (None, None),
        };

        Self {
            mode,
            user_id,
            team_id,
            deployment_id,
            shell_version,
            client_type,
            client_version,
            subscription_tier,
        }
    }
}


static TELEMETRY_CLIENT: OnceLock<Mutex<Option<TelemetryClient>>> = OnceLock::new();

/// Returns `true` when telemetry mode is `Enabled`.
/// Used by `log_event` — product analytics events only fire in `Enabled` mode.
pub fn is_enabled() -> bool {
    TELEMETRY_CLIENT
        .get()
        .and_then(|m| m.lock().ok())
        .is_some_and(|g| g.as_ref().is_some_and(|c| c.mode.is_enabled()))
}

/// Returns `true` when telemetry mode is `Enabled` or `SessionMetrics`.
/// Used by `session_metrics` — lifecycle events fire in both modes.
pub fn is_session_metrics_enabled() -> bool {
    TELEMETRY_CLIENT
        .get()
        .and_then(|m| m.lock().ok())
        .is_some_and(|g| g.as_ref().is_some_and(|c| c.mode.session_metrics_enabled()))
}

pub struct UserContext {
    pub country: String,
    pub language: String,
    pub timestamp: String,
}

impl UserContext {
    pub fn collect() -> Self {
        let default_language = whoami::Language::En(whoami::Country::Any);
        let lang = whoami::langs()
            .ok()
            .and_then(|mut langs| langs.next())
            .unwrap_or(default_language);
        Self {
            country: lang.country().to_string(),
            language: lang.to_string(),
            timestamp: Local::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

/// Core telemetry emitter — no-op (all external telemetry removed).
pub async fn track(_event_name: &str, _request_id: &str, _ctx: &UserContext, _metadata: Metadata) {
    // All external telemetry (product events, Mixpanel) has been removed.
    // This function is kept as a no-op to avoid breaking callers.
}

/// Sync user profile — no-op (Mixpanel removed).
pub fn sync_profile() {
    // Mixpanel profile sync has been removed.
}

/// Initialize telemetry client. Safe to call multiple times.
///
/// - `Disabled` → no client
/// - `SessionMetrics` → client active (only `session_metrics::*` events fire)
/// - `Enabled` → client active (all events fire)
///
/// `shell_version` is stamped into every event payload as `shell_version`
/// (legacy field name preserved for analytics continuity); shell passes its
/// own `CARGO_PKG_VERSION`. `http_client` is owned by the caller (typically
/// shell's `shared_client()`) so the shared TLS-warmed pool is reused for
/// telemetry posts.
pub fn init(
    config: TelemetryConfig,
    mode: TelemetryMode,
    user_id: Option<String>,
    team_id: Option<String>,
    deployment_key: Option<String>,
    origin_client: Option<OriginClientInfo>,
    shell_version: String,
    subscription_tier: Option<String>,
    http_client: reqwest::Client,
) {
    let lock = TELEMETRY_CLIENT.get_or_init(|| Mutex::new(None));
    let mut guard = lock.lock().unwrap_or_else(|err| err.into_inner());
    *guard = if mode.is_disabled() {
        None
    } else {
        Some(TelemetryClient::from_config(
            config,
            mode,
            user_id,
            team_id,
            deployment_key,
            origin_client,
            shell_version,
            subscription_tier,
            http_client,
        ))
    };
    drop(guard);
    sync_profile();
}

/// Re-initialize the telemetry client if it was not created at startup
/// (e.g. because auth was not yet available). No-op when the client
/// is already set, so safe to call unconditionally after auth succeeds.
pub fn init_if_needed(
    config: TelemetryConfig,
    mode: TelemetryMode,
    user_id: Option<String>,
    team_id: Option<String>,
    deployment_key: Option<String>,
    origin_client: Option<OriginClientInfo>,
    shell_version: String,
    subscription_tier: Option<String>,
    http_client: reqwest::Client,
) {
    if mode.is_disabled() {
        return;
    }
    let lock = TELEMETRY_CLIENT.get_or_init(|| Mutex::new(None));
    let mut guard = lock.lock().unwrap_or_else(|err| err.into_inner());
    if guard.is_none() {
        *guard = Some(TelemetryClient::from_config(
            config,
            mode,
            user_id,
            team_id,
            deployment_key,
            origin_client,
            shell_version,
            subscription_tier,
            http_client,
        ));
        drop(guard);
        sync_profile();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shell events must still strip to their bare suffix, byte-for-byte
    /// identical to the previous `strip_prefix("sage-shell-")` behavior.
    #[test]
    fn event_value_strips_shell_prefix() {
        assert_eq!(event_value("sage-shell-turn"), "turn");
        assert_eq!(
            event_value("sage-shell-trace_upload_attempted"),
            "trace_upload_attempted"
        );
    }

    /// Workspace events strip their own prefix to the same bare suffix.
    #[test]
    fn event_value_strips_workspace_prefix() {
        assert_eq!(event_value("sage-workspace-turn"), "turn");
    }

    /// SessionMetrics must not attempt Mixpanel profile engage — sync_profile
    /// is a no-op unless mode is fully Enabled.
    #[test]
    fn sync_profile_is_noop_in_session_metrics_mode() {
        // No tokio runtime here BY DESIGN: if the gate wrongly falls through,
        // sync_profile's tokio::spawn panics and fails this test. Converting
        // this to #[tokio::test] would silently turn it into theater.
        assert!(
            tokio::runtime::Handle::try_current().is_err(),
            "this test must run without a tokio runtime"
        );
        // Clear the global client even if an assert below panics.
        struct ClearClient;
        impl Drop for ClearClient {
            fn drop(&mut self) {
                let lock = TELEMETRY_CLIENT.get_or_init(|| Mutex::new(None));
                *lock.lock().unwrap_or_else(|err| err.into_inner()) = None;
            }
        }
        let _clear = ClearClient;

        // Mixpanel configured, but no events endpoint: the global must never
        // carry a live funnel out of this test.
        let cfg = TelemetryConfig {
            mixpanel_enabled: true,
            mixpanel_token: Some("test-token".into()),
            events_url: None,
            events_api_key: None,
            ..TelemetryConfig::default()
        };
        init(
            cfg,
            TelemetryMode::SessionMetrics,
            Some("user-1".into()),
            None,
            None,
            None,
            "0.0.0-test".into(),
            None,
            reqwest::Client::new(),
        );
        // Explicit call must no-op too (init already invoked it once).
        sync_profile();
        assert!(
            is_session_metrics_enabled(),
            "client must be live for session metrics"
        );
        assert!(!is_enabled(), "product analytics must stay off");
    }

    /// Names without a known emitter prefix pass through unchanged (preserves
    /// the old `unwrap_or(event_name)` fallback).
    #[test]
    fn event_value_passes_through_unprefixed() {
        assert_eq!(event_value("turn"), "turn");
        assert_eq!(event_value(""), "");
    }

    /// Only the leading emitter prefix is stripped; a suffix that itself looks
    /// like another prefix is left intact.
    #[test]
    fn event_value_strips_only_leading_prefix() {
        assert_eq!(event_value("sage-shell-workspace-x"), "workspace-x");
    }

    /// The stripper recovers the bare suffix for every origin the emitter can
    /// produce — ties `event_value` to `EmitterOrigin::event_prefix`.
    #[test]
    fn event_value_round_trips_every_emitter_prefix() {
        for origin in EmitterOrigin::ALL {
            let name = format!("{}my_event", origin.event_prefix());
            assert_eq!(event_value(&name), "my_event");
        }
    }

    /// Mixpanel `subscription_tier` must be a stable snake_case key. Free
    /// users arrive as CCP display `"Free"` or JWT-fallback `"free"`; both
    /// must land as `"free"` (not omitted / not `"Free"`).
    #[test]
    fn normalize_tier_maps_display_and_claim_names() {
        assert_eq!(normalize_tier("Free"), "free");
        assert_eq!(normalize_tier("free"), "free");
        assert_eq!(normalize_tier("SuperGrok"), "supergrok");
        assert_eq!(normalize_tier("SuperGrok Heavy"), "supersage_heavy");
        assert_eq!(normalize_tier("supersage_heavy"), "supersage_heavy");
        assert_eq!(normalize_tier("X Basic"), "x_basic");
        assert_eq!(normalize_tier("X Premium+"), "x_premium_plus");
        assert_eq!(normalize_tier("X Premium"), "x_premium");
        assert_eq!(normalize_tier("SuperGrok Lite"), "supersage_lite");
        // API key is a dedicated Mixpanel segment — never free.
        assert_eq!(normalize_tier("API Key"), "api_key");
        assert_eq!(normalize_tier("api_key"), "api_key");
    }

    /// `event_value`'s first-match-wins over `EmitterOrigin::ALL` is only
    /// correct because the emitter prefixes are mutually exclusive: no origin's
    /// `event_prefix()` is a prefix of another's. If that invariant ever broke
    /// (e.g. a future `"sage-shell-ext-"` origin), an earlier `ALL` entry could
    /// strip a shorter prefix first and yield the wrong `event_value`. Pin the
    /// invariant so adding such a variant fails the suite rather than silently
    /// corrupting analytics.
    #[test]
    fn emitter_prefixes_are_mutually_exclusive() {
        for a in EmitterOrigin::ALL {
            for b in EmitterOrigin::ALL {
                if a != b {
                    assert!(
                        !a.event_prefix().starts_with(b.event_prefix()),
                        "{a:?} prefix {:?} must not start with {b:?} prefix {:?}",
                        a.event_prefix(),
                        b.event_prefix(),
                    );
                }
            }
        }
    }
}
