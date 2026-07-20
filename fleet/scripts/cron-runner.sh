#!/bin/bash
# Cron Runner — Nexus 无头模式定时任务调度器
# 通过系统 crontab 调用，替代 CCB 的内置 Cron
#
# 设置方法:
#   crontab -e
#   # 每 4 小时快速巡检
#   0 */4 * * * cd /e/Git仓库/Agent-SYS/fleet && bash scripts/cron-runner.sh quick
#   # 每天 00:00 全量巡检
#   0 0 * * * cd /e/Git仓库/Agent-SYS/fleet && bash scripts/cron-runner.sh full
#   # 每天 08:00 趋势扫描
#   0 8 * * * cd /e/Git仓库/Agent-SYS/fleet && bash scripts/cron-runner.sh trend

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FLEET_DIR="$(dirname "$SCRIPT_DIR")"
MODE="${1:-quick}"
TIMESTAMP=$(date -Iseconds)
LOG_DIR="$FLEET_DIR/logs"

mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/cron-${MODE}-$(date +%Y%m%d).log"

log() {
    echo "[$(date -Iseconds)] $1" | tee -a "$LOG_FILE"
}

log "Cron Runner started — mode: $MODE"

case "$MODE" in
    quick)
        log "Running quick health check..."
        bash "$SCRIPT_DIR/health-check.sh" --json >> "$LOG_FILE" 2>&1
        log "Quick health check completed"
        ;;

    full)
        log "Running full health check..."
        bash "$SCRIPT_DIR/health-check.sh" --full --json >> "$LOG_FILE" 2>&1

        # Generate markdown report as well
        bash "$SCRIPT_DIR/health-check.sh" --full > "$FLEET_DIR/docs/fleet-health.md" 2>> "$LOG_FILE"
        log "Full health check completed — report saved to docs/fleet-health.md"
        ;;

    trend)
        log "Running trend scan..."
        # For trend scanning, we'd use Nexus headless mode:
        # nexus agent headless -p "分析当前技术趋势，生成舰队趋势报告" --output-format markdown
        # But nexus must be installed first
        if command -v nexus &> /dev/null; then
            log "nexus CLI found, running trend scan..."
            # nexus agent headless -p "Run advisor analysis: scan tech trends and generate fleet trend report" \
            #   --cwd "$FLEET_DIR/.." \
            #   --output "$FLEET_DIR/docs/fleet-trends.md" 2>> "$LOG_FILE"
            log "(nexus headless mode integration pending — nexus binary path confirmation needed)"
        else
            log "WARNING: nexus CLI not found. Install Nexus first:"
            log "  cd ../nexus && cargo build -p nexus-bin --release"
            log "  cp target/release/nexus /usr/local/bin/nexus"
        fi
        ;;

    *)
        log "ERROR: Unknown mode '$MODE'. Use: quick, full, or trend"
        exit 1
        ;;
esac

log "Cron Runner finished"

# Cleanup logs older than 30 days
find "$LOG_DIR" -name "*.log" -mtime +30 -delete 2>/dev/null || true
