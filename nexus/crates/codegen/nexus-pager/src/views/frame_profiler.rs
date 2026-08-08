//! Debug-only per-segment frame profiler (`NEXUS_FPS=full`).
//!
//! The FPS HUD reports the wall-clock of the entire `draw_frame` call (render
//! closure + buffer diff + flush), but not where the time goes. This profiler
//! splits each frame into:
//!
//! - `render`: the render closure body inside `draw_frame`
//! - `agent`: the `AgentView::draw` calls within that closure
//! - `flush`: `total - render` (buffer diff + term-writer handoff)
//!
//! Enabled only when `NEXUS_FPS=full`; any other truthy value keeps the plain
//! FPS overlay as the only consumer. Disabled cost is a few ns-scale
//! `Instant`/`Cell` ops per frame (far below the frame budget); only the
//! accumulation + write path is gated on the bool. Appends one summary line
//! per second to a log file (`NEXUS_FRAME_PROFILE_LOG`, default temp-dir
//! `nexus-frame-profile.log`) — never stderr, because the TUI's render
//! pipeline owns stderr on Windows and would be corrupted by stray text.

use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Per-second segment accumulator that appends a line to the profile log.
pub struct FrameProfiler {
    enabled: bool,
    frames: u64,
    total_ns: u128,
    render_ns: u128,
    agent_ns: u128,
    last_print: Instant,
}

impl Default for FrameProfiler {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameProfiler {
    pub fn new() -> Self {
        Self::with_env(std::env::var("NEXUS_FPS").ok().as_deref())
    }

    /// `env` is the raw `NEXUS_FPS` value; the profiler activates only on the
    /// exact value `full` (case/whitespace insensitive), leaving every other
    /// truthy value to the plain FPS overlay.
    fn with_env(env: Option<&str>) -> Self {
        let enabled = env.is_some_and(|v| v.trim().eq_ignore_ascii_case("full"));
        Self {
            enabled,
            frames: 0,
            total_ns: 0,
            render_ns: 0,
            agent_ns: 0,
            last_print: Instant::now(),
        }
    }

    /// Whether the per-segment profiler is active.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Accumulate one frame's segment timings. No-op unless enabled.
    pub fn record(&mut self, total: Duration, render: Duration, agent: Duration) {
        if !self.enabled {
            return;
        }
        self.frames += 1;
        self.total_ns += total.as_nanos();
        self.render_ns += render.as_nanos();
        self.agent_ns += agent.as_nanos();
        if self.last_print.elapsed() >= Duration::from_secs(1) {
            let f = self.frames.max(1) as f64;
            let mean_ms = |ns: u128| ns as f64 / f / 1e6;
            let line = format!(
                "[frame] n={:.0} total={:.1}ms render={:.1}ms flush={:.1}ms agent={:.1}ms",
                f,
                mean_ms(self.total_ns),
                mean_ms(self.render_ns),
                mean_ms(self.total_ns.saturating_sub(self.render_ns)),
                mean_ms(self.agent_ns),
            );
            // Append-only; write failures are silent — this is a diagnostic,
            // never worth crashing the render path over.
            if let Ok(mut fh) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path())
            {
                let _ = writeln!(fh, "{line}");
            }
            self.frames = 0;
            self.total_ns = 0;
            self.render_ns = 0;
            self.agent_ns = 0;
            self.last_print = Instant::now();
        }
    }
}

/// Where per-second summary lines are appended. `NEXUS_FRAME_PROFILE_LOG`
/// overrides the default temp-dir path.
fn log_path() -> PathBuf {
    std::env::var_os("NEXUS_FRAME_PROFILE_LOG")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("nexus-frame-profile.log"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_by_default_and_only_full_env_enables() {
        assert!(!FrameProfiler::new().enabled());
        for v in ["1", "true", "0"] {
            assert!(!FrameProfiler::with_env(Some(v)).enabled(), "{v:?}");
        }
        for v in ["full", "FULL", " full "] {
            assert!(FrameProfiler::with_env(Some(v)).enabled(), "{v:?}");
        }
        assert!(!FrameProfiler::with_env(None).enabled());
    }

    #[test]
    fn record_is_a_noop_while_disabled() {
        let mut p = FrameProfiler::new();
        p.record(Duration::from_millis(10), Duration::from_millis(9), Duration::from_millis(8));
        assert_eq!(p.frames, 0);
        assert_eq!(p.total_ns, 0);
    }
}
