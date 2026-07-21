use agent_client_protocol as acp;

use crate::agent::config::ModelEntry;
use crate::auth::PreferredAuthMethod;

/// Shared, live handle to the agent's current ACP auth method id.
///
/// `Arc` so a clone can cross the per-session-thread boundary at spawn; the
/// `ArcSwapOption` interior lets the agent's `authenticate` handler publish a
/// new method that every running session's per-turn auth gate observes on its
/// next turn -- no re-spawn. `None` until the first `authenticate`. Auth is
/// process-global (one user, one `AuthManager`), so all sessions sharing one
/// cell is correct.
pub(crate) type SharedAuthMethodId = std::sync::Arc<arc_swap::ArcSwapOption<acp::AuthMethodId>>;

/// Construct a [`SharedAuthMethodId`]. `None` is the pre-`authenticate` state.
pub(crate) fn new_shared_auth_method_id(initial: Option<acp::AuthMethodId>) -> SharedAuthMethodId {
    std::sync::Arc::new(arc_swap::ArcSwapOption::new(
        initial.map(std::sync::Arc::new),
    ))
}

/// Env var that, when set, advertises `xai.api_key` as a viable auth method.
///
/// Kept as a constant so test code and the production check stay in sync.
pub const XAI_API_KEY_ENV_VAR: &str = "NEXUS_API_KEY";

/// Legacy env var name. Checked as a fallback when `NEXUS_API_KEY` is not set,
/// so existing deployments that use the old name keep working.
pub const LEGACY_XAI_API_KEY_ENV_VAR: &str = "XAI_API_KEY";

/// Read the API key from the environment.
///
/// Checks `XAI_API_KEY` first, then falls back to the legacy
/// `NEXUS_CODE_XAI_API_KEY` for backward compatibility.
pub fn read_xai_api_key_env() -> Result<String, std::env::VarError> {
    std::env::var(XAI_API_KEY_ENV_VAR).or_else(|_| std::env::var(LEGACY_XAI_API_KEY_ENV_VAR))
}

/// Returns `true` if either `XAI_API_KEY` or `NEXUS_CODE_XAI_API_KEY` is set.
pub fn has_xai_api_key_env() -> bool {
    read_xai_api_key_env().is_ok()
}

/// Whether `xai.api_key` should be advertised (and pushed FIRST) when building
/// the `auth_methods` list at `initialize()` time.
///
/// Regression: `xai.api_key` must stay first when only per-model credentials
/// exist (no global `XAI_API_KEY`). Deferring it made BYOK users hit the login
/// screen because the pager uses `auth_methods.first()` for startup metadata.
///
/// [`build_auth_methods`] consumes this predicate and pins the ordering;
/// its tests catch call-site and predicate regressions.
///
/// Probes `std::env` at call time and consults each `ModelEntry` for a
/// resolvable api_key/env_key -- both inputs can change between calls, so the
/// result is not cached.
///
/// `disable_api_key_auth` (`[nexus_com_config] disable_api_key_auth` /
/// `NEXUS_DISABLE_API_KEY_AUTH`) is the admin kill switch: when true the
/// method is never advertised, regardless of available credentials, so
/// `XAI_API_KEY` can't bypass a deployment's forced IdP login.
pub fn should_advertise_xai_api_key<'a, I>(disable_api_key_auth: bool, models: I) -> bool
where
    I: IntoIterator<Item = &'a ModelEntry>,
{
    if disable_api_key_auth {
        return false;
    }
    has_xai_api_key_env() || models.into_iter().any(ModelEntry::has_own_credentials)
}

/// Inputs to [`build_auth_methods`].
///
/// Booleans are computed by the caller (`MvpAgent::initialize()`) because they
/// depend on async side effects (token refresh) and shared mutable state
/// (`AuthManager`). The list-construction logic itself is pure so it can be
/// unit-tested without any of that machinery.
pub struct AuthMethodsBuildInputs<'a> {
    /// True if `xai.api_key` should be advertised AT ALL. Caller computes via
    /// [`should_advertise_xai_api_key`]. When `preferred_method` is `Oidc`,
    /// this is ignored (API key is never advertised under that pin).
    pub has_external_api_key: bool,
    /// True if a cached session token is available (either present at startup
    /// or recovered via silent refresh).
    pub has_cached_token: bool,
    /// True if enterprise OIDC is configured. Mutually exclusive with the
    /// default `sage.local` method.
    pub has_enterprise_oidc: bool,
    /// Required when `has_enterprise_oidc` is true; ignored otherwise.
    pub enterprise_oidc_issuer: Option<&'a str>,
    /// Optional display label for the login method (`sage.local` or `oidc`).
    pub login_label: Option<&'a str>,
    /// True if `nexus_com_config.oauth2` is configured (e.g. `NEXUS_OAUTH2_ISSUER`
    /// env var). When all three of `has_enterprise_oidc`, `has_oauth2`, and
    /// `has_auth_provider_command` are false, no interactive login method is
    /// advertised because there is no mechanism to complete an interactive login.
    pub has_oauth2: bool,
    /// True if `nexus_com_config.auth_provider_command` is configured (sets
    /// `meta.external_provider = true` on the `sage.local` method).
    pub has_auth_provider_command: bool,
    /// Config pin (`[auth] preferred_method`). `None` keeps multi-method
    /// fallthrough; `Some` is fail-closed (only that method family).
    pub preferred_method: Option<PreferredAuthMethod>,
}

/// Output of [`build_auth_methods`].
pub struct BuiltAuthMethods {
    /// Auth methods in advertised order. ORDER IS THE CONTRACT: the pager's
    /// `startup_auth_metadata()` reads `methods.first()` to decide whether
    /// interactive login is needed.
    pub methods: Vec<acp::AuthMethod>,
    /// The default `auth_method_id` to install on the agent. When unpinned,
    /// `cached_token` wins over `xai.api_key` when both are present. When
    /// pinned, only the preferred method may appear; `None` means unavailable
    /// (fail auth — no cross-method fallthrough).
    pub default_auth_method_id: Option<acp::AuthMethodId>,
}

/// Build the `auth_methods` list and default `auth_method_id` from
/// pre-computed inputs.
///
/// Sage simplified auth: ONLY API key authentication is supported.
/// No OAuth2, OIDC, cached token, or interactive login — just
/// `XAI_API_KEY` env var or per-model `api_key` in config.toml.
///
/// When API key credentials are available, returns `[xai.api_key]`.
/// Otherwise, returns an empty list (caller shows a setup prompt).
pub fn build_auth_methods(inputs: AuthMethodsBuildInputs<'_>) -> BuiltAuthMethods {
    let has_external_api_key = inputs.has_external_api_key;

    if !has_external_api_key {
        return BuiltAuthMethods {
            methods: Vec::new(),
            default_auth_method_id: None,
        };
    }

    BuiltAuthMethods {
        methods: vec![xai_api_key_auth_method()],
        default_auth_method_id: Some(acp::AuthMethodId::new(XAI_API_KEY_METHOD_ID)),
    }
}

fn push_interactive_login(
    methods: &mut Vec<acp::AuthMethod>,
    has_enterprise_oidc: bool,
    enterprise_oidc_issuer: Option<&str>,
    login_label: Option<&str>,
    has_oauth2: bool,
    has_auth_provider_command: bool,
) {
    // When no interactive login mechanism is available (no OIDC, no OAuth2,
    // no external auth provider), do NOT advertise any interactive method.
    // Advertising sage.local with nothing behind it causes the pager to
    // trigger an interactive login that will always fail.
    if !has_enterprise_oidc && !has_oauth2 && !has_auth_provider_command {
        tracing::info!("auth: no interactive login method available, skipping sage.local");
        return;
    }
    if has_enterprise_oidc {
        // Caller invariant: `enterprise_oidc_issuer` MUST be `Some(...)` when
        // `has_enterprise_oidc` is true. Production callers derive both from
        // the same `cfg.nexus_com_config.oidc` Option, so the inconsistent
        // `(true, None)` combination is a programmer error -- panic loudly
        // (matches the original `cfg.nexus_com_config.oidc.as_ref().unwrap()`
        // call in `MvpAgent::initialize()` before this refactor).
        let issuer = enterprise_oidc_issuer
            .expect("enterprise_oidc_issuer is required when has_enterprise_oidc is true");
        methods.push(oidc_auth_method(issuer, login_label));
    } else {
        methods.push(nexus_com_auth_method(login_label, has_auth_provider_command));
    }
}

/// ACP session auth method. Use `is_session_based_method` for classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethodKind {
    XaiApiKey,
    CachedToken,
    GrokCom,
    Oidc,
    Unknown,
}

impl AuthMethodKind {
    pub fn from_id(id: &acp::AuthMethodId) -> Self {
        match id.0.as_ref() {
            XAI_API_KEY_METHOD_ID => Self::XaiApiKey,
            CACHED_TOKEN_AUTH_METHOD_ID => Self::CachedToken,
            NEXUS_COM_METHOD_ID => Self::GrokCom,
            OIDC_METHOD_ID => Self::Oidc,
            _ => Self::Unknown,
        }
    }

    /// API key auth: no auth.json, no refresh, no user interaction.
    pub fn is_api_key(self) -> bool {
        matches!(self, Self::XaiApiKey)
    }

    /// `true` for session-based methods (cached_token, sage.local, oidc).
    pub fn is_session_based(self) -> bool {
        matches!(self, Self::CachedToken | Self::GrokCom | Self::Oidc)
    }

    /// Requires user interaction (browser, OIDC redirect, or external auth command).
    pub fn needs_interactive_login(self) -> bool {
        matches!(self, Self::GrokCom | Self::Oidc)
    }

    pub fn auth_error_message(self) -> &'static str {
        if self.is_session_based() {
            AUTH_ERROR_SESSION_EXPIRED
        } else {
            AUTH_ERROR_API_KEY
        }
    }
}

/// `true` for session-based ACP methods (cached_token, sage.local, oidc).
pub fn is_session_based_method(method_id: &acp::AuthMethodId) -> bool {
    AuthMethodKind::from_id(method_id).is_session_based()
}

/// Per-model BYOK status: whether the selected model carries its own
/// `[model.*]` `api_key`/`env_key`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelByok {
    /// Model has its own per-model key (not refreshable).
    Byok,
    /// Model has no per-model key (session auth governs).
    NotByok,
    /// Config couldn't be loaded/parsed — BYOK status indeterminate.
    Unknown,
}

impl ModelByok {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Byok => "byok",
            Self::NotByok => "not_byok",
            Self::Unknown => "unknown",
        }
    }
}

/// Whether this session+model uses a refreshable session token.
///
/// Gates on stable inputs, not `Credentials.auth_type`: that field collapses
/// to `ApiKey` when the session-token cache is momentarily empty and
/// `XAI_API_KEY` is set, which demoted live OIDC sessions to non-refreshable
/// api-key mode and 401'd every prompt until restart. `model_byok` still
/// excludes genuine per-model BYOK, whose keys are not refreshable.
///
/// `Unknown` (BYOK status indeterminate — config currently unparseable, no
/// sampling config yet, or the per-model memo was cleared) must **not** demote
/// a live session to non-refreshable api-key mode: that re-sends the stale
/// buffered token on every turn and 401s with `bad-credentials` until restart
/// (the stale-token regression this gate addresses; fall back rather than
/// demote on `Unknown`). It refreshes when `endpoint_is_first_party` — the
/// request targets a first-party host (cli-chat-proxy / first-party API),
/// where sending the session token cannot leak to a third-party BYOK
/// endpoint. A definite `NotByok` always refreshes (it only ever routes to
/// the session endpoint); a definite `Byok` never does.
pub fn session_token_auth_gate(
    is_session_based_method: bool,
    model_byok: ModelByok,
    endpoint_is_first_party: bool,
) -> bool {
    is_session_based_method
        && match model_byok {
            ModelByok::NotByok => true,
            ModelByok::Byok => false,
            ModelByok::Unknown => endpoint_is_first_party,
        }
}

pub const AUTH_ERROR_SESSION_EXPIRED: &str =
    "Session expired. Run `sage login` to re-authenticate.";

pub const AUTH_ERROR_API_KEY: &str = "Authentication failed. Run `nexus login`, set NEXUS_API_KEY, or add api_key to config.toml.";

/// Next ACP method id when `cached_token` cannot proceed (missing / expired /
/// legacy WebLogin), or `None` when fallthrough is forbidden.
///
/// Unpinned: prefer non-interactive `xai.api_key` when advertiseable, else
/// interactive `sage.local`.
///
/// Pinned `oidc`: **no** fallthrough to api_key — return `None` so the caller
/// fails auth. Pinned `api_key` should not reach this path (cached_token is
/// not advertised).
pub fn method_id_after_cached_token_unavailable(
    has_external_api_key: bool,
    preferred_method: Option<PreferredAuthMethod>,
) -> Option<&'static str> {
    match preferred_method {
        Some(PreferredAuthMethod::Oidc) | Some(PreferredAuthMethod::ApiKey) => None,
        None => Some(if has_external_api_key {
            XAI_API_KEY_METHOD_ID
        } else {
            NEXUS_COM_METHOD_ID
        }),
    }
}

/// Error when `preferred_method=api_key` but no key/BYOK credentials exist.
pub const PREFERRED_API_KEY_UNAVAILABLE: &str = "preferred_method=api_key but no API key is configured (set NEXUS_API_KEY or model api_key/env_key in config.toml).";

/// Error when `preferred_method=oidc` but the session path cannot proceed.
pub const PREFERRED_OIDC_UNAVAILABLE: &str =
    "preferred_method=oidc but no session is available. Run `sage login` to authenticate.";

pub const XAI_API_KEY_METHOD_ID: &str = "xai.api_key";
pub fn xai_api_key_auth_method() -> acp::AuthMethod {
    acp::AuthMethod::Agent(
        acp::AuthMethodAgent::new(
            acp::AuthMethodId::new(XAI_API_KEY_METHOD_ID),
            "xai.api_key".to_string(),
        )
        .description(Some(format!(
            "{XAI_API_KEY_ENV_VAR} or api_key/env_key in config.toml"
        ))),
    )
}

pub const CACHED_TOKEN_AUTH_METHOD_ID: &str = "cached_token";
pub fn cached_token_auth_method() -> acp::AuthMethod {
    acp::AuthMethod::Agent(
        acp::AuthMethodAgent::new(
            acp::AuthMethodId::new(CACHED_TOKEN_AUTH_METHOD_ID),
            "cached_token".to_string(),
        )
        .description(Some("Cached token from ~/.nexus/auth.json".to_string())),
    )
}

pub const NEXUS_COM_METHOD_ID: &str = "sage.local";

/// xAI OAuth2/OIDC auth. Method id `"sage.local"` kept for ACP wire-compat.
pub fn nexus_com_auth_method(
    label: Option<&str>,
    has_auth_provider_command: bool,
) -> acp::AuthMethod {
    let name = label.unwrap_or("Nexus");
    let meta = if has_auth_provider_command {
        let mut m = acp::Meta::new();
        m.insert("external_provider".to_owned(), serde_json::json!(true));
        Some(m)
    } else {
        None
    };
    acp::AuthMethod::Agent(
        acp::AuthMethodAgent::new(acp::AuthMethodId::new(NEXUS_COM_METHOD_ID), name.to_string())
            .description(Some(format!("Sign in with {name}")))
            .meta(meta),
    )
}

pub const OIDC_METHOD_ID: &str = "oidc";
pub fn oidc_auth_method(issuer: &str, label: Option<&str>) -> acp::AuthMethod {
    let name = label
        .map(|l| l.to_string())
        .unwrap_or_else(|| format!("Single sign-on ({})", issuer));
    acp::AuthMethod::Agent(
        acp::AuthMethodAgent::new(acp::AuthMethodId::new(OIDC_METHOD_ID), name.clone())
            .description(Some(format!("Sign in with {name}"))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::config::{Config, resolve_model_list};
    use agent_client_protocol as acp;
    use serial_test::serial;

    /// When API-key credentials are advertiseable, fall through from a dead
    /// `cached_token` to non-interactive `xai.api_key` (not browser OAuth).
    /// Covers the both-advertised case (`has_cached_token` true at initialize
    /// but session later missing/expired/legacy): advertise order still puts
    /// `xai.api_key` first, while `default_auth_method_id` prefers session;
    /// after session fails, this helper must still pick `xai.api_key`.
    #[test]
    fn after_cached_token_unavailable_prefers_api_key_when_advertiseable() {
        assert_eq!(
            method_id_after_cached_token_unavailable(true, None),
            Some(XAI_API_KEY_METHOD_ID),
        );
    }

    /// No advertiseable API-key credentials → interactive `sage.local`.
    #[test]
    fn after_cached_token_unavailable_falls_to_nexus_com_without_api_key() {
        assert_eq!(
            method_id_after_cached_token_unavailable(false, None),
            Some(NEXUS_COM_METHOD_ID),
        );
    }

    /// Pinned methods never fall through across the api_key ↔ oidc boundary.
    #[test]
    fn after_cached_token_unavailable_fails_closed_when_pinned() {
        assert_eq!(
            method_id_after_cached_token_unavailable(true, Some(PreferredAuthMethod::Oidc)),
            None,
        );
        assert_eq!(
            method_id_after_cached_token_unavailable(true, Some(PreferredAuthMethod::ApiKey)),
            None,
        );
    }

    /// Classifier matrix for all auth method variants.
    #[test]
    fn auth_method_kind_classifier_matrix() {
        let session_methods = [
            CACHED_TOKEN_AUTH_METHOD_ID,
            NEXUS_COM_METHOD_ID,
            OIDC_METHOD_ID,
        ];
        for method_id in session_methods {
            let id = acp::AuthMethodId::new(method_id);
            let kind = AuthMethodKind::from_id(&id);
            assert!(
                kind.is_session_based(),
                "{method_id}: kind must be session-based"
            );
            assert!(
                is_session_based_method(&id),
                "{method_id}: wrapper must agree"
            );
        }
        let api_id = acp::AuthMethodId::new(XAI_API_KEY_METHOD_ID);
        let api_kind = AuthMethodKind::from_id(&api_id);
        assert!(!api_kind.is_session_based());
        assert!(api_kind.is_api_key());
        assert!(!is_session_based_method(&api_id));
        assert!(!is_session_based_method(&acp::AuthMethodId::new(
            "unknown-method"
        )));
    }

    use nexus_test_support::EnvGuard;

    // ── Helpers ─────────────────────────────────────────────────────────

    /// Default inputs to `build_auth_methods` representing a session-only user
    /// with no API key anywhere. Tests override only the fields they care
    /// about.
    fn default_inputs() -> AuthMethodsBuildInputs<'static> {
        AuthMethodsBuildInputs {
            has_external_api_key: false,
            has_cached_token: false,
            has_enterprise_oidc: false,
            enterprise_oidc_issuer: None,
            login_label: None,
            has_oauth2: false,
            has_auth_provider_command: false,
            preferred_method: None,
        }
    }

    fn method_ids(built: &BuiltAuthMethods) -> Vec<&str> {
        built.methods.iter().map(|m| m.id().0.as_ref()).collect()
    }

    fn default_id(built: &BuiltAuthMethods) -> Option<&str> {
        built
            .default_auth_method_id
            .as_ref()
            .map(|id| id.0.as_ref())
    }

    fn first_kind(methods: &[acp::AuthMethod]) -> Option<AuthMethodKind> {
        methods.first().map(|m| AuthMethodKind::from_id(m.id()))
    }

    // build_auth_methods regression: pin production call-site ordering.
    // Reordering so `xai.api_key` is after login methods must fail the tests below.

    /// BYOK with only per-model `env_key` must list `xai.api_key` first.
    #[test]
    fn enterprise_byok_first_method_is_xai_api_key() {
        let inputs = AuthMethodsBuildInputs {
            has_external_api_key: true, // enterprise user with resolved per-model env_key
            has_cached_token: false,
            ..default_inputs()
        };
        let built = build_auth_methods(inputs);

        assert_eq!(
            first_kind(&built.methods),
            Some(AuthMethodKind::XaiApiKey),
            "BYOK enterprise-style: auth_methods.first() MUST be xai.api_key \
             (deferred-to-last ordering sends users to the login screen)",
        );
        assert_eq!(
            built
                .default_auth_method_id
                .as_ref()
                .map(|id| id.0.as_ref()),
            Some(XAI_API_KEY_METHOD_ID),
        );
        // Cross-check with the pager-side predicate: the first method must
        // not require interactive login, which is the exact condition the
        // pager's `startup_auth_metadata()` uses.
        assert!(
            !AuthMethodKind::from_id(built.methods[0].id()).needs_interactive_login(),
            "first method MUST NOT need interactive login when xai.api_key is available",
        );
    }

    /// Simplified auth model: cached_token / OIDC / interactive login are
    /// no longer supported. When API key is available, only xai.api_key is
    /// advertised; the presence of a cached token does not add extra methods.
    #[test]
    fn byok_with_cached_token_ignores_cached_token() {
        let inputs = AuthMethodsBuildInputs {
            has_external_api_key: true,
            has_cached_token: true,
            ..default_inputs()
        };
        let built = build_auth_methods(inputs);

        assert_eq!(
            first_kind(&built.methods),
            Some(AuthMethodKind::XaiApiKey),
            "API key must be the only method even when cached_token flag is set",
        );
        // Simplified: cached_token is never advertised.
        assert_eq!(built.methods.len(), 1);
        assert_eq!(
            built
                .default_auth_method_id
                .as_ref()
                .map(|id| id.0.as_ref()),
            Some(XAI_API_KEY_METHOD_ID),
        );
    }

    /// Session-only user without API key: no methods are advertised
    /// (simplified auth — only API key is supported).
    #[test]
    fn session_only_user_without_api_key_has_no_methods() {
        let inputs = AuthMethodsBuildInputs {
            has_external_api_key: false,
            has_cached_token: true,
            ..default_inputs()
        };
        let built = build_auth_methods(inputs);

        assert!(built.methods.is_empty());
        assert!(built.default_auth_method_id.is_none());
    }

    /// Brand-new user (no API key, no cached token): no methods advertised.
    /// The caller handles the empty case (e.g. showing a setup prompt).
    #[test]
    fn fresh_user_without_api_key_has_no_methods() {
        let built = build_auth_methods(default_inputs());

        assert!(built.methods.is_empty());
        assert!(built.default_auth_method_id.is_none());
    }

    /// Enterprise OIDC flag is ignored in simplified auth — only API key
    /// is advertised when available.
    #[test]
    fn enterprise_oidc_flag_is_ignored_only_api_key_advertised() {
        let inputs = AuthMethodsBuildInputs {
            has_external_api_key: true,
            has_cached_token: false,
            has_enterprise_oidc: true,
            enterprise_oidc_issuer: Some("https://sso.example.com"),
            ..default_inputs()
        };
        let built = build_auth_methods(inputs);

        assert_eq!(first_kind(&built.methods), Some(AuthMethodKind::XaiApiKey));
        // OIDC is never advertised in simplified auth.
        assert_eq!(built.methods.len(), 1);
    }

    /// Auth provider command flag is ignored in simplified auth.
    /// When no API key is available, no methods are advertised.
    #[test]
    fn auth_provider_command_is_ignored_no_methods_without_api_key() {
        let inputs = AuthMethodsBuildInputs {
            has_auth_provider_command: true,
            login_label: Some("Acme Corp"),
            ..default_inputs()
        };
        let built = build_auth_methods(inputs);

        // Simplified: no interactive login methods — only API key matters.
        assert!(built.methods.is_empty());
    }

    /// Admin kill switch (`disable_api_key_auth`): no methods when API key
    /// auth is disabled, regardless of available credentials.
    #[test]
    #[serial]
    fn disable_api_key_auth_suppresses_xai_api_key_method() {
        let _set = EnvGuard::set(XAI_API_KEY_ENV_VAR, "xai-external-key");
        let cfg = Config::default();
        let models = resolve_model_list(&cfg, None);

        // Flag off: API key advertised.
        assert!(should_advertise_xai_api_key(false, models.values()));

        // Flag on: never advertised, regardless of credentials.
        let has_external_api_key = should_advertise_xai_api_key(true, models.values());
        assert!(!has_external_api_key);
        let built = build_auth_methods(AuthMethodsBuildInputs {
            has_external_api_key,
            ..default_inputs()
        });
        assert!(
            !built
                .methods
                .iter()
                .any(|m| AuthMethodKind::from_id(m.id()) == AuthMethodKind::XaiApiKey),
            "xai.api_key must not be advertised when disable_api_key_auth is set",
        );
        // Simplified: no login method fallback — empty methods when API key
        // auth is disabled.
        assert!(built.methods.is_empty());
        assert!(built.default_auth_method_id.is_none());
    }

    /// Legacy auth token (WebLogin) is no longer supported — cached_token
    /// flag is ignored. No API key means no methods.
    #[test]
    #[serial]
    fn nexus_login_legacy_token_ignored_no_api_key_no_methods() {
        use crate::auth::{AuthManager, AuthMode, GrokAuth, SageComConfig};

        let _g1 = EnvGuard::unset("NEXUS_AUTH_PATH");
        let _g2 = EnvGuard::unset(XAI_API_KEY_ENV_VAR);

        let legacy_token = GrokAuth {
            key: "legacy-relay-token".into(),
            auth_mode: AuthMode::WebLogin,
            create_time: chrono::Utc::now(),
            user_id: "legacy-user".into(),
            email: Some("legacy@example.com".into()),
            oidc_issuer: None,
            oidc_client_id: None,
            refresh_token: None,
            expires_at: None,
            ..GrokAuth::test_default()
        };

        let legacy_json = serde_json::to_string(&legacy_token).expect("serialize legacy token");
        let _g = EnvGuard::set("NEXUS_AUTH", &legacy_json);

        let dir = tempfile::tempdir().unwrap();
        let cfg = SageComConfig::default();
        let mgr = AuthManager::new(dir.path(), cfg);
        let current = mgr.current();
        assert!(
            current.is_some(),
            "legacy token in NEXUS_AUTH env MUST be loaded directly",
        );

        let has_cached_token = mgr.current().is_some();
        assert!(has_cached_token);

        // Simplified auth: cached_token flag is ignored. No API key → no methods.
        let built = build_auth_methods(AuthMethodsBuildInputs {
            has_external_api_key: false,
            has_cached_token,
            ..default_inputs()
        });

        assert!(
            built.methods.is_empty(),
            "simplified auth: cached_token is not advertised, no methods without API key",
        );
        assert!(built.default_auth_method_id.is_none());
    }

    /// No legacy token and no API key: empty methods list (simplified auth).
    #[test]
    #[serial]
    fn no_legacy_token_means_no_methods() {
        use crate::auth::{AuthManager, SageComConfig};

        let _g1 = EnvGuard::unset("NEXUS_AUTH");
        let _g2 = EnvGuard::unset("NEXUS_AUTH_PATH");

        let dir = tempfile::tempdir().unwrap();
        let cfg = SageComConfig::default();
        let mgr = AuthManager::new(dir.path(), cfg);
        assert!(mgr.current().is_none());

        let built = build_auth_methods(AuthMethodsBuildInputs {
            has_external_api_key: false,
            has_cached_token: mgr.current().is_some(),
            ..default_inputs()
        });
        assert!(
            built.methods.is_empty(),
            "no API key and no cached token: no auth methods available",
        );
    }

    // ── preferred_method pin (fail-closed) ──────────────────────────────

    #[test]
    fn pin_api_key_with_key_only_advertises_api_key() {
        let built = build_auth_methods(AuthMethodsBuildInputs {
            has_external_api_key: true,
            has_cached_token: true,
            preferred_method: Some(PreferredAuthMethod::ApiKey),
            ..default_inputs()
        });
        assert_eq!(method_ids(&built), vec![XAI_API_KEY_METHOD_ID]);
        assert_eq!(default_id(&built), Some(XAI_API_KEY_METHOD_ID));
    }

    #[test]
    fn pin_api_key_without_key_fails_closed_even_with_session() {
        let built = build_auth_methods(AuthMethodsBuildInputs {
            has_external_api_key: false,
            has_cached_token: true,
            preferred_method: Some(PreferredAuthMethod::ApiKey),
            ..default_inputs()
        });
        assert!(built.methods.is_empty());
        assert!(built.default_auth_method_id.is_none());
    }

    /// OIDC pin without API key: fails closed (OIDC is no longer supported).
    #[test]
    fn pin_oidc_fails_closed_in_simplified_auth() {
        let built = build_auth_methods(AuthMethodsBuildInputs {
            has_external_api_key: true,
            has_cached_token: true,
            preferred_method: Some(PreferredAuthMethod::Oidc),
            ..default_inputs()
        });
        // Simplified auth: OIDC pin fails closed (no OIDC methods available).
        assert!(built.methods.is_empty());
        assert!(built.default_auth_method_id.is_none());
    }

    /// OIDC pin without session: fails closed in simplified auth.
    #[test]
    fn pin_oidc_without_session_fails_closed_in_simplified_auth() {
        let built = build_auth_methods(AuthMethodsBuildInputs {
            has_external_api_key: true,
            has_cached_token: false,
            preferred_method: Some(PreferredAuthMethod::Oidc),
            ..default_inputs()
        });
        // Simplified auth: OIDC pin fails closed (no interactive login).
        assert!(built.methods.is_empty());
        assert!(built.default_auth_method_id.is_none());
    }
}
