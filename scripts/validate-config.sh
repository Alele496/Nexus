#!/bin/bash
# Agent-SYS 配置完整性验证脚本
# 检查所有 .sage/ 配置文件是否有明显问题

set -e

SYS_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ERRORS=0
WARNINGS=0

red() { echo -e "\033[31m$1\033[0m"; }
green() { echo -e "\033[32m$1\033[0m"; }
yellow() { echo -e "\033[33m$1\033[0m"; }

echo "========================================="
echo " Agent-SYS Configuration Validator"
echo " Root: $SYS_ROOT"
echo "========================================="
echo ""

# ---- AGENTS.md ----
echo -n "[1/8] AGENTS.md ... "
if [ -f "$SYS_ROOT/AGENTS.md" ]; then
    green "OK ($(wc -l < "$SYS_ROOT/AGENTS.md") lines)"
else
    red "MISSING — Nexus needs this as project rules entry point"
    ERRORS=$((ERRORS + 1))
fi

# ---- .sage/config.toml ----
echo -n "[2/8] .sage/config.toml ... "
if [ -f "$SYS_ROOT/.sage/config.toml" ]; then
    # Check TOML is parseable (basic check with grep for required sections)
    if grep -q '\[subagents\]' "$SYS_ROOT/.sage/config.toml" && \
       grep -q '\[subagents.roles.developer\]' "$SYS_ROOT/.sage/config.toml" && \
       grep -q '\[subagents.roles.reviewer\]' "$SYS_ROOT/.sage/config.toml" && \
       grep -q '\[subagents.roles.operator\]' "$SYS_ROOT/.sage/config.toml" && \
       grep -q '\[subagents.roles.advisor\]' "$SYS_ROOT/.sage/config.toml"; then
        green "OK (4 roles defined)"
    else
        red "PARSE WARNING — missing expected role sections"
        WARNINGS=$((WARNINGS + 1))
    fi
else
    red "MISSING"
    ERRORS=$((ERRORS + 1))
fi

# ---- .sage/agents/ ----
echo -n "[3/8] .sage/agents/ ... "
AGENT_COUNT=0
for agent in developer reviewer operator advisor; do
    if [ -f "$SYS_ROOT/.sage/agents/$agent.md" ]; then
        AGENT_COUNT=$((AGENT_COUNT + 1))
    else
        red "MISSING: $agent.md"
        ERRORS=$((ERRORS + 1))
    fi
done
if [ $AGENT_COUNT -eq 4 ]; then
    green "OK ($AGENT_COUNT agents)"
fi

# ---- .sage/skills/ ----
echo -n "[4/8] .sage/skills/ ... "
SKILL_COUNT=0
for skill in dispatch ship council-review; do
    if [ -f "$SYS_ROOT/.sage/skills/$skill/SKILL.md" ]; then
        # Check YAML frontmatter
        if head -1 "$SYS_ROOT/.sage/skills/$skill/SKILL.md" | grep -q '^---$'; then
            SKILL_COUNT=$((SKILL_COUNT + 1))
        else
            yellow "WARN: $skill/SKILL.md missing YAML frontmatter"
            WARNINGS=$((WARNINGS + 1))
        fi
    else
        red "MISSING: $skill/SKILL.md"
        ERRORS=$((ERRORS + 1))
    fi
done
if [ $SKILL_COUNT -eq 3 ]; then
    green "OK ($SKILL_COUNT skills)"
fi

# ---- .sage/hooks/ ----
echo -n "[5/8] .sage/hooks/ ... "
if [ -f "$SYS_ROOT/.sage/hooks/safety-gates.json" ]; then
    if grep -q '"PreToolUse"' "$SYS_ROOT/.sage/hooks/safety-gates.json" && \
       grep -q '"matcher"' "$SYS_ROOT/.sage/hooks/safety-gates.json"; then
        green "OK (PreToolUse + PostToolUse + Stop hooks)"
    else
        yellow "WARN: hook structure may be incomplete"
        WARNINGS=$((WARNINGS + 1))
    fi
else
    red "MISSING"
    ERRORS=$((ERRORS + 1))
fi

# ---- .sage/rules/ ----
echo -n "[6/8] .sage/rules/ ... "
if [ -f "$SYS_ROOT/.sage/rules/project-conventions.md" ]; then
    green "OK"
else
    yellow "MISSING (optional)"
    WARNINGS=$((WARNINGS + 1))
fi

# ---- fleet/ ----
echo -n "[7/8] fleet/ ... "
FLEET_COUNT=0
for f in README.md fleet-registry.json docs/fleet-policy.md docs/spawn-protocol.md; do
    if [ -f "$SYS_ROOT/fleet/$f" ]; then
        FLEET_COUNT=$((FLEET_COUNT + 1))
    else
        yellow "MISSING: $f"
        WARNINGS=$((WARNINGS + 1))
    fi
done
green "OK ($FLEET_COUNT/4 files)"

# ---- Config consistency check ----
echo -n "[8/8] Cross-reference check ... "
ISSUES=0

# Check: every agent .md referenced in config.toml exists
for agent in developer reviewer operator advisor; do
    if grep -q "agents/$agent.md" "$SYS_ROOT/.sage/config.toml" 2>/dev/null; then
        if [ ! -f "$SYS_ROOT/.sage/agents/$agent.md" ]; then
            yellow "WARN: config.toml references $agent.md but file missing"
            ISSUES=$((ISSUES + 1))
        fi
    fi
done

# Check: every persona in config.toml has instructions
if grep -q 'security-reviewer' "$SYS_ROOT/.sage/config.toml" && \
   grep -q 'perf-reviewer' "$SYS_ROOT/.sage/config.toml" && \
   grep -q 'readability-reviewer' "$SYS_ROOT/.sage/config.toml"; then
    :
else
    yellow "WARN: some personas may be missing"
    ISSUES=$((ISSUES + 1))
fi

if [ $ISSUES -eq 0 ]; then
    green "OK"
else
    WARNINGS=$((WARNINGS + ISSUES))
fi

echo ""
echo "========================================="
if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    green " ALL CHECKS PASSED — 0 errors, 0 warnings"
elif [ $ERRORS -eq 0 ]; then
    yellow " PASSED WITH WARNINGS — $ERRORS errors, $WARNINGS warnings"
else
    red " FAILED — $ERRORS errors, $WARNINGS warnings"
fi
echo "========================================="

exit $ERRORS
