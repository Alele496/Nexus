//! Provider presets shared between the first-run setup wizard (`nexus-bin`)
//! and the F2 settings panel (`nexus-pager`).
//!
//! A "provider" here is a UI-layer onboarding bundle: a display name, a
//! default API endpoint, an `api_backend` and a curated model list. The
//! runtime has no provider concept — each `[model.*]` entry carries its own
//! `base_url`/`api_backend`/`api_key`. These presets only make it easy to
//! populate those entries.

/// 提供商预设：包含名称、默认模型列表、API 端点、后端类型等信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderPreset {
    /// 显示名称
    pub name: &'static str,
    /// 默认 API 端点
    pub base_url: &'static str,
    /// API 后端类型（`chat_completions` | `responses` | `messages`）
    pub api_backend: &'static str,
    /// 是否支持推理深度调节
    pub supports_reasoning: bool,
    /// 模型列表: (model_id, display_name, context_window)
    pub models: &'static [(&'static str, &'static str, u64)],
}

/// 内置提供商预设。
pub const PROVIDERS: &[ProviderPreset] = &[
    ProviderPreset {
        name: "DeepSeek",
        base_url: "https://api.deepseek.com/v1",
        api_backend: "chat_completions",
        supports_reasoning: true,
        models: &[
            ("deepseek-v4-pro", "DeepSeek V4 Pro — 最强推理，1M 上下文 (推荐)", 1_000_000),
            ("deepseek-v4-flash", "DeepSeek V4 Flash — 更快响应，日常开发", 1_000_000),
        ],
    },
    ProviderPreset {
        name: "OpenAI",
        base_url: "https://api.openai.com/v1",
        api_backend: "chat_completions",
        supports_reasoning: true,
        models: &[
            ("gpt-5.2", "GPT-5.2 — 最强综合能力 (推荐)", 128_000),
            ("gpt-5.1", "GPT-5.1 — 平衡性能与速度", 128_000),
            ("gpt-5-mini", "GPT-5 Mini — 轻量快速，日常任务", 128_000),
        ],
    },
    ProviderPreset {
        name: "Anthropic (Claude)",
        base_url: "https://api.anthropic.com/v1",
        api_backend: "messages",
        supports_reasoning: false,
        models: &[
            ("claude-opus-4-7", "Claude Opus 4.7 — 最强推理，适合复杂任务 (推荐)", 200_000),
            ("claude-sonnet-4-6", "Claude Sonnet 4.6 — 快速响应的主力模型", 200_000),
            ("claude-haiku-4-5", "Claude Haiku 4.5 — 极速轻量，日常任务", 200_000),
        ],
    },
    ProviderPreset {
        name: "自定义 (OpenAI 兼容 API)",
        base_url: "",
        api_backend: "chat_completions",
        supports_reasoning: false,
        models: &[],
    },
];

impl ProviderPreset {
    /// Look up a preset by exact display name.
    pub fn by_name(name: &str) -> Option<&'static ProviderPreset> {
        PROVIDERS.iter().find(|p| p.name == name)
    }

    /// Match the active provider against a resolved inference base URL.
    ///
    /// The active provider is derived from `[endpoints] xai_api_base_url`.
    /// A preset matches when its `base_url` equals the configured endpoint
    /// (after trimming a trailing `/`). Returns `None` for the Custom preset
    /// (empty `base_url`) or when no preset matches.
    pub fn match_base_url(base_url: &str) -> Option<&'static ProviderPreset> {
        let trimmed = base_url.trim_end_matches('/');
        PROVIDERS.iter().find(|p| !p.base_url.is_empty() && p.base_url.trim_end_matches('/') == trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_has_name_and_valid_backend() {
        for p in PROVIDERS {
            assert!(!p.name.is_empty(), "provider name must not be empty");
            assert!(
                matches!(p.api_backend, "chat_completions" | "responses" | "messages"),
                "invalid api_backend {:?} for {}",
                p.api_backend,
                p.name
            );
        }
    }

    #[test]
    fn builtin_presets_have_models() {
        for p in PROVIDERS {
            if p.base_url.is_empty() {
                // Custom preset may carry no curated models.
                continue;
            }
            assert!(!p.models.is_empty(), "{} preset must list models", p.name);
            for (id, name, ctx) in p.models {
                assert!(!id.is_empty(), "model id must not be empty for {}", p.name);
                assert!(!name.is_empty());
                assert!(*ctx > 0);
            }
        }
    }

    #[test]
    fn by_name_finds_deepseek() {
        let p = ProviderPreset::by_name("DeepSeek").expect("DeepSeek preset");
        assert_eq!(p.base_url, "https://api.deepseek.com/v1");
        assert_eq!(p.api_backend, "chat_completions");
    }

    #[test]
    fn match_base_url_ignores_trailing_slash() {
        assert_eq!(
            ProviderPreset::match_base_url("https://api.deepseek.com/v1"),
            ProviderPreset::by_name("DeepSeek")
        );
        assert_eq!(
            ProviderPreset::match_base_url("https://api.deepseek.com/v1/"),
            ProviderPreset::by_name("DeepSeek")
        );
        assert!(ProviderPreset::match_base_url("https://example.com/v1").is_none());
    }
}
