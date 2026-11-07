"""Stop hook: the main session posts a CHECK-IN of eight tables at least hourly."""

import json
import re
import sys
from datetime import datetime, timedelta, timezone

MAX_AGE = timedelta(minutes=60)
HEADINGS = [
    ("Roadmap", r"1\.\s*Roadmap"),
    ("Running", r"2\.\s*Running"),
    ("Changed since last check-in", r"3\.\s*Changed since last check-in"),
    ("Cycle metrics", r"4\.\s*Cycle metrics"),
    ("Decisions I took without you", r"5\.\s*Decisions I took without you"),
    ("Suggested process improvements", r"6\.\s*Suggested process improvements"),
    ("Needs you", r"7\.\s*Needs you"),
    ("Next", r"8\.\s*Next"),
]
STALE = ("Blocked: more than 60 minutes since the last CHECK-IN; post the check-in (eight tables: Roadmap, "
         "Running, Changed since last, Cycle metrics, Decisions without the owner, Suggested process "
         "improvements, Needs you, Next)")


def text_of(message):
    content = message.get("content")
    if isinstance(content, str):
        return content
    parts = [b.get("text", "") for b in content or [] if isinstance(b, dict) and b.get("type") == "text"]
    return "\n".join(parts)


def parse_time(value):
    try:
        moment = datetime.fromisoformat(str(value).replace("Z", "+00:00"))
    except ValueError:
        return None
    return moment if moment.tzinfo else moment.replace(tzinfo=timezone.utc)


def read_transcript(path):
    last, newest = "", None
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            try:
                row = json.loads(line)
            except ValueError:
                continue
            if not isinstance(row, dict):
                continue
            message = row.get("message") if isinstance(row.get("message"), dict) else row
            if (message.get("role") or row.get("type")) != "assistant":
                continue
            text = text_of(message).strip()
            if not text:
                continue
            last = text
            moment = parse_time(row.get("timestamp"))
            if "CHECK-IN" in text and moment is not None:
                newest = moment
    return last, newest


def missing_table(text):
    # The check-in may open with checklists ("**Right now: ...**", "**Phases**" with - [x] / - [ ] lines)
    # before table 1; everything before the table 1 heading is skipped.
    first = re.compile(r"^\s*\*\*\s*" + HEADINGS[0][1], re.M | re.I).search(text)
    if not first:
        return 1, HEADINGS[0][0]
    position = first.start()
    for number, (name, pattern) in enumerate(HEADINGS, 1):
        found = re.compile(r"^\s*\*\*\s*" + pattern, re.M | re.I).search(text, position)
        if not found:
            return number, name
        rest = text[found.end():]
        following = re.search(r"^\s*\*\*\s*\d\.", rest, re.M)
        section = rest[:following.start()] if following else rest
        if not re.search(r"^\s*\|", section, re.M):
            return number, name
        position = found.end()
    return None


def main():
    try:
        data = json.load(sys.stdin)
        path = data.get("transcript_path")
        if data.get("stop_hook_active") or not path or "/subagents/" in path.replace("\\", "/"):
            return 0
        last, newest = read_transcript(path)
    except (ValueError, OSError, AttributeError):
        return 0
    if last.startswith("**CHECK-IN"):
        gap = missing_table(last)
        if gap is None:
            return 0
        print("Blocked: the CHECK-IN is missing table %d (%s) or has them out of order; post all eight "
              "tables in the locked order" % gap, file=sys.stderr)
        return 2
    if newest is None or datetime.now(timezone.utc) - newest > MAX_AGE:
        print(STALE, file=sys.stderr)
        return 2
    return 0


sys.exit(main())
