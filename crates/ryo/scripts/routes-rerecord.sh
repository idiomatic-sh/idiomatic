#!/bin/bash
# routes-rerecord.sh <language> "<reason>" [<corpus-name> <corpus-root>...]
#
# Records the frozen rows of one language (languages/<language>/tests/frozen/) from its OLD READER: the digests of every fixture
# sample and relation, the completeness of every sample, the node-table relations and the words of the positions, and what the
# language states of itself; with a corpus name and roots, the corpus digests too (rows.<name>.digests). It REFUSES to write when the
# old reader and the walk differ anywhere: a difference is never frozen. Run it where cargo may run (under build/codex/lane.sh).
#
# The reason is written into the header of every file it writes, and it must be the reason of the commit that carries the new
# files: a re-record is a decision, like the cases reference, and the diff of the digest files is its review.
set -eu
if [ "$#" -lt 2 ] || [ -z "$2" ]; then
    echo "usage: routes-rerecord.sh <language> \"<reason>\" [<corpus-name> <corpus-root>...]" >&2
    exit 2
fi
language=$1
reason=$2
shift 2
cd "$(dirname "$0")/.."
if [ "$#" -ge 2 ]; then
    name=$1
    shift
    roots=$(IFS=:; echo "$*")
    export RYO_ROUTES_CORPUS="$roots" RYO_ROUTES_CORPUS_NAME="$name"
fi
export RYO_ROUTES_RECORD="$reason"
cargo test --offline -p "idiomatic-sh-ryo-$language" --features fixtures --test routes
echo "recorded languages/$language/tests/frozen/ (reason: $reason): put the reason in the commit message"
