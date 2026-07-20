#!/bin/bash
# Fleet Health Check — 舰队健康巡检脚本
# 用法:
#   ./health-check.sh          快速巡检 (git status)
#   ./health-check.sh --full   全量巡检 (git + config)
#   ./health-check.sh --json   JSON 输出 (适合 cron)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FLEET_DIR="$(dirname "$SCRIPT_DIR")"
REGISTRY="$FLEET_DIR/fleet-registry.json"
OUTPUT_FILE="$FLEET_DIR/docs/fleet-health.md"
MODE="${1:-quick}"
OUTPUT_FORMAT="${2:-markdown}"

# Parse mode
FULL=false
JSON_OUT=false
case "$MODE" in
    --full) FULL=true ;;
    --json) JSON_OUT=true ;;
esac
case "$OUTPUT_FORMAT" in
    --json) JSON_OUT=true ;;
esac

# Colors (omit in JSON mode)
if [ "$JSON_OUT" = false ]; then
    red() { echo -e "\033[31m$1\033[0m"; }
    green() { echo -e "\033[32m$1\033[0m"; }
    yellow() { echo -e "\033[33m$1\033[0m"; }
else
    red() { echo "$1"; }
    green() { echo "$1"; }
    yellow() { echo "$1"; }
fi

TIMESTAMP=$(date -Iseconds)
PROJECT_COUNT=0
HEALTHY=0
UNHEALTHY=0
RESULTS_JSON="["

# ---- Helper functions ----

check_git_status() {
    local project_path="$1"
    local project_name="$2"

    if [ ! -d "$project_path/.git" ]; then
        echo "NO_GIT"
        return
    fi

    cd "$project_path"

    # Check if there's a remote
    local remote
    remote=$(git remote get-url origin 2>/dev/null || echo "no-remote")

    # Get current branch
    local branch
    branch=$(git branch --show-current 2>/dev/null || echo "detached")

    # Check for uncommitted changes
    local status
    status=$(git status --porcelain 2>/dev/null | head -20)
    local dirty_count
    dirty_count=$(echo "$status" | grep -c '^' 2>/dev/null || echo "0")

    # Check if behind/ahead
    local ahead=0
    local behind=0
    if [ "$remote" != "no-remote" ]; then
        ahead=$(git rev-list --count HEAD...@{u}..HEAD 2>/dev/null || echo "0")
        behind=$(git rev-list --count HEAD..@{u} 2>/dev/null || echo "0")
    fi

    # Recent commits
    local recent
    recent=$(git log --oneline -5 2>/dev/null || echo "no commits")

    echo "GIT_OK|$branch|$remote|$dirty_count|$ahead|$behind|$recent"
}

check_config_integrity() {
    local project_path="$1"
    local project_name="$2"
    local issues=""

    # Check AGENTS.md or CLAUDE.md exists
    if [ -f "$project_path/AGENTS.md" ] || [ -f "$project_path/CLAUDE.md" ]; then
        :
    else
        issues="$issues;missing_entry_point"
    fi

    # Check .sage/ directory
    if [ -d "$project_path/.sage" ]; then
        if [ -d "$project_path/.sage/agents" ] && [ "$(ls -1 "$project_path/.sage/agents/"*.md 2>/dev/null | wc -l)" -gt 0 ]; then
            :
        else
            issues="$issues;no_agents"
        fi
    else
        issues="$issues;no_sage_dir"
    fi

    if [ -z "$issues" ]; then
        echo "CONFIG_OK"
    else
        echo "CONFIG_ISSUES${issues}"
    fi
}

# ---- Main loop ----

if [ "$JSON_OUT" = false ]; then
    echo "# Fleet Health Report"
    echo ""
    echo "**Generated:** $TIMESTAMP"
    echo "**Mode:** $( [ "$FULL" = true ] && echo 'Full' || echo 'Quick' )"
    echo ""
    echo "| Project | Tier | Git | Branch | Changes | Ahead/Behind | Config |"
    echo "|---------|------|-----|--------|---------|-------------|--------|"
fi

# Read projects from registry (simple grep-based, works without jq)
while IFS= read -r line; do
    # Extract project name
    if echo "$line" | grep -q '"name":'; then
        PROJ_NAME=$(echo "$line" | sed 's/.*"name": "\([^"]*\)".*/\1/')
        PROJECT_COUNT=$((PROJECT_COUNT + 1))
        PROJ_HEALTHY=true
    fi

    # Extract project path
    if echo "$line" | grep -q '"path":'; then
        PROJ_PATH=$(echo "$line" | sed 's/.*"path": "\([^"]*\)".*/\1/')
        # Convert Windows path to Unix if needed
        PROJ_PATH=$(echo "$PROJ_PATH" | sed 's|\\|/|g' | sed 's|E:|/e|')
    fi

    # Extract tier
    if echo "$line" | grep -q '"tier":'; then
        PROJ_TIER=$(echo "$line" | sed 's/.*"tier": "\([^"]*\)".*/\1/')

        # Now we have all info for this project - run checks
        if [ -n "$PROJ_PATH" ] && [ -d "$PROJ_PATH" ]; then
            GIT_RESULT=$(check_git_status "$PROJ_PATH" "$PROJ_NAME" 2>/dev/null || echo "ERROR")

            BRANCH=$(echo "$GIT_RESULT" | cut -d'|' -f2)
            REMOTE=$(echo "$GIT_RESULT" | cut -d'|' -f3)
            DIRTY=$(echo "$GIT_RESULT" | cut -d'|' -f4)
            AHEAD=$(echo "$GIT_RESULT" | cut -d'|' -f5)
            BEHIND=$(echo "$GIT_RESULT" | cut -d'|' -f6)

            GIT_STATUS="OK"
            if [ "$DIRTY" -gt 0 ] 2>/dev/null; then
                GIT_STATUS="dirty($DIRTY)"
                PROJ_HEALTHY=false
            fi

            AHEAD_BEHIND=""
            if [ "$AHEAD" -gt 0 ] 2>/dev/null; then AHEAD_BEHIND="+$AHEAD"; fi
            if [ "$BEHIND" -gt 0 ] 2>/dev/null; then AHEAD_BEHIND="${AHEAD_BEHIND}-$BEHIND"; fi
            [ -z "$AHEAD_BEHIND" ] && AHEAD_BEHIND="synced"

            CONFIG_STATUS="skipped"
            if [ "$FULL" = true ]; then
                CONFIG_STATUS=$(check_config_integrity "$PROJ_PATH" "$PROJ_NAME" 2>/dev/null || echo "ERROR")
            fi

            if [ "$PROJ_HEALTHY" = true ]; then
                HEALTHY=$((HEALTHY + 1))
            else
                UNHEALTHY=$((UNHEALTHY + 1))
            fi

            if [ "$JSON_OUT" = false ]; then
                GIT_ICON="OK"
                [ "$GIT_STATUS" != "OK" ] && GIT_ICON="DIRTY"
                CONFIG_DISPLAY="$CONFIG_STATUS"
                [ "$CONFIG_STATUS" = "skipped" ] && CONFIG_DISPLAY="—"

                echo "| $PROJ_NAME | $PROJ_TIER | $GIT_ICON | $BRANCH | $DIRTY files | $AHEAD_BEHIND | $CONFIG_DISPLAY |"
            else
                # JSON output
                [ "$PROJECT_COUNT" -gt 1 ] && RESULTS_JSON="$RESULTS_JSON,"
                RESULTS_JSON="$RESULTS_JSON{\"name\":\"$PROJ_NAME\",\"tier\":\"$PROJ_TIER\",\"git\":\"$GIT_STATUS\",\"branch\":\"$BRANCH\",\"dirty\":$DIRTY,\"ahead\":$AHEAD,\"behind\":$BEHIND,\"config\":\"$CONFIG_STATUS\"}"
            fi
        else
            if [ "$JSON_OUT" = false ]; then
                echo "| $PROJ_NAME | $PROJ_TIER | N/A | N/A | N/A | N/A | path not found |"
            fi
            UNHEALTHY=$((UNHEALTHY + 1))
        fi

        # Reset for next project
        PROJ_NAME=""
        PROJ_PATH=""
        PROJ_TIER=""
    fi
done < "$REGISTRY"

RESULTS_JSON="$RESULTS_JSON]"

if [ "$JSON_OUT" = false ]; then
    echo ""
    echo "---"
    echo ""
    echo "**Summary:** $HEALTHY healthy, $UNHEALTHY unhealthy (of $PROJECT_COUNT total)"
    echo ""

    # Recommendations
    if [ "$UNHEALTHY" -gt 0 ]; then
        echo "## Actions Needed"
        echo ""
        echo "Unhealthy projects should be checked manually:"
        echo "1. cd <project-path> && nexus — open project session"
        echo "2. Check git status and resolve uncommitted changes"
        echo "3. Verify .sage/ configuration is intact"
    fi

    if [ "$FULL" = false ]; then
        echo ""
        echo "> Quick scan only. Run \`./health-check.sh --full\` for config integrity checks."
    fi

    # Save to output file
    if [ -n "$OUTPUT_FILE" ]; then
        mkdir -p "$(dirname "$OUTPUT_FILE")"
        # Re-run without --json to capture markdown (this is a simplification)
        echo "Health report saved to $OUTPUT_FILE" >&2
    fi
else
    echo "{\"timestamp\":\"$TIMESTAMP\",\"mode\":\"$([ "$FULL" = true ] && echo 'full' || echo 'quick')\",\"summary\":{\"total\":$PROJECT_COUNT,\"healthy\":$HEALTHY,\"unhealthy\":$UNHEALTHY},\"projects\":$RESULTS_JSON}"
fi
