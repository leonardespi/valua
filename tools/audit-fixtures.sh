#!/usr/bin/env bash
# Scans all success-fixture expected.lua files for non-decimal integer literals.
#
# Exit 0 — clean.  Exit 1 — violations found (or --fix failed).
#
# Usage:
#   ./tools/audit-fixtures.sh          # audit only
#   ./tools/audit-fixtures.sh --fix    # audit + regenerate via UPDATE_FIXTURES

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FIXTURE_ROOT="$ROOT/tests/fixtures"
FIX="${1:-}"

echo "=== valua fixture decimal-emission audit ==="
printf "Workspace: %s\n\n" "$ROOT"

found=0
checked=0

# ── Discovery pass ────────────────────────────────────────────────────────────
while IFS= read -r -d '' expected_file; do
    dir=$(dirname "$expected_file")
    fixture=$(basename "$dir")

    # Skip sub-directories that hold error fixtures (no Lua emission)
    [[ "$fixture" == "errors" || "$fixture" == "backends" ]] && continue

    checked=$((checked + 1))

    # Scan non-comment Lua lines for 0x… or 0b… prefixes.
    # Grep returns exit 1 when no match; the "|| true" absorbs that.
    violations=$(grep -nP '0[xXbB][0-9a-fA-F_]+' "$expected_file" \
        | grep -vP '^\d+:\s*--' \
        || true)

    if [[ -n "$violations" ]]; then
        printf "FAIL  %s/expected.lua\n" "$fixture"
        while IFS= read -r line; do
            printf "        %s\n" "$line"
        done <<< "$violations"
        echo ""
        found=$((found + 1))
    fi
done < <(find "$FIXTURE_ROOT" -maxdepth 2 -name "expected.lua" -print0 | sort -z)

# ── Report ────────────────────────────────────────────────────────────────────
printf "Checked: %d expected.lua file(s)\n\n" "$checked"

if [[ "$found" -eq 0 ]]; then
    echo "PASS  All success fixtures use plain-decimal integer emission."
    exit 0
fi

printf "FAIL  %d fixture(s) contain non-decimal integer literals.\n\n" "$found"

if [[ "$FIX" == "--fix" ]]; then
    echo "Regenerating via UPDATE_FIXTURES=1 …"
    cd "$ROOT"
    UPDATE_FIXTURES=1 cargo test update_all_fixtures -- --ignored 2>&1
    echo ""
    echo "Review changes:"
    git -C "$ROOT" diff tests/fixtures/ || true
else
    cat <<'EOF'
To auto-regenerate run one of:

    UPDATE_FIXTURES=1 cargo test update_all_fixtures -- --ignored
    ./tools/audit-fixtures.sh --fix
EOF
fi

exit 1
