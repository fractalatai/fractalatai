#!/usr/bin/python3
"""Print a reminder when the cross-instrument holders pass (#77) is due (last run > --days ago).

Silent when not due, so it can run from a SessionStart hook and the lat-sync skill.
  check_due.py            # default 90 days
  check_due.py --days 60
  check_due.py --hook     # SessionStart hook: JSON with a systemMessage (shown to the user) and additionalContext
"""

import argparse
import datetime as dt
import json
import os

STAMP = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "last_run")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--days", type=int, default=90)
    ap.add_argument("--hook", action="store_true")
    args = ap.parse_args()
    try:
        last = dt.date.fromisoformat(open(STAMP).read().strip())
        age = (dt.date.today() - last).days
        msg = (f"Reminder: the cross-instrument holders pass (#77) last ran {last} ({age} days ago). "
               "Run the cross-instrument-holders skill.") if age > args.days else None
    except (OSError, ValueError):
        msg = "Reminder: the cross-instrument holders pass (#77) has never been run. Run the cross-instrument-holders skill."
    if not msg:
        return
    if args.hook:
        print(json.dumps({"systemMessage": msg,
                          "hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": msg}}))
    else:
        print(msg)


if __name__ == "__main__":
    main()
