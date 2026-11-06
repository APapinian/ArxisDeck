#!/bin/sh
# SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
# SPDX-License-Identifier: GPL-3.0-or-later
# Quality gates from ROADMAP.md ("Cross-milestone quality gates"), runnable offline:
#   1. cargo fmt --check        2. cargo clippy (warnings are errors)
#   3. cargo test               4. QML: the interface loads and draws (--screenshot)
#   5. controller self-test     (scripted gamepad through every core flow, simulated store)
#   6. THIRD-PARTY-LICENSES.md matches the crates compiled in
# The screenshot and self-test never touch the system: they use the simulated store.
set -eu
cd "$(dirname "$0")/.."
out=build/check
mkdir -p "$out"

echo "== 1/6 format";  cargo fmt --check
echo "== 2/6 clippy";  cargo clippy --offline --all-targets -- -D warnings
echo "== 3/6 tests";   cargo test --offline --quiet
echo "== 4/6 QML loads and draws"
cargo build --offline --quiet
./target/debug/arxis-deck --screenshot "$out/screenshot.png"
echo "== 5/6 controller self-test"
./target/debug/arxis-deck --self-test "$out/self-test.json"
python3 -I - "$out/self-test.json" <<'EOF'
import json, sys
steps = json.load(open(sys.argv[1]))["steps"]
failed = [s["step"] for s in steps if not s["passed"]]
print(f"{len(steps) - len(failed)}/{len(steps)} steps passed")
sys.exit(1 if failed else 0)
EOF
echo "== 6/6 third-party licences"
python3 -I scripts/third_party_licenses.py --check
echo "All gates passed."
