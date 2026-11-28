#!/bin/bash
# sync-findings.sh "MESSAGE": copies the contract findings from the evidence repository into the findings worktree
# (branch claude/ryo-contract-sheets, off the perf tip 2a207464c), commits and pushes; the draft PR into
# claude/tm-one-path updates itself. Destination in the tree: crates/ryo/design/contracts/.
set -euo pipefail
SRC=/workspaces/ryo-poc-evidence/next/build/ports/stage2
W=/workspaces/ryo-contract-findings
DST=$W/crates/ryo/design/contracts
mkdir -p "$DST/inputs/discovery" "$DST/inputs/mapping"
rsync -a --delete --exclude __pycache__ --exclude inputs "$SRC/contracts/" "$DST/"
rsync -a --delete "$SRC/discovery-inputs/" "$DST/inputs/discovery/"
rsync -a --delete "$SRC/mapping/" "$DST/inputs/mapping/"
git -C "$W" add crates/ryo/design/contracts
if git -C "$W" diff --cached --quiet; then echo "nothing new"; exit 0; fi
git -C "$W" commit -q -m "$1

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git -C "$W" push -q origin claude/ryo-contract-sheets
git -C "$W" log -1 --format='%h %s'
