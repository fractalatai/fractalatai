# Session docs

This practice was adopted from sertantai-legal on 2026-09-30. The good example is `2026-09-27-issue-166.md` in legal's `.claude/sessions/`.

## Scope: a session is a stretch of work, not an issue

- **Name:** `<topic>/YYYY-MM-DD-<what>.md`, with `issue: N` in the frontmatter when an issue drives it. Older docs use `MM-DD-YY-<name>.md` and stay as they are.
- **Sessions aren't mirrors of GitHub issues** (Jason, 2026-10-01). One issue can span several sessions, and one session can touch several issues.
- **Flow, not ping-pong:**
  - Work in one active session.
  - To move on, **close it**: hand every open item to the pending or suspended session where it'll actually be done (or drop it with a reason), then open the next.
  - Don't bounce between sessions mid-task.
- **Split rule:** when unrelated work comes up mid-session and would need its own Todo list, open a **pending** stub for it and add a one-line pointer. Don't absorb a second project into the running doc.
- **Size:** ~50–150 lines is healthy. Past ~200 lines, or with more than a handful of dated sections on different topics, close and move on.

## Structure

```yaml
---
session: "Issue #72: Hohfeld correlatives"   # quote values containing ':' or '"'
status: pending | active | suspended | closed
opened: 2026-09-30
closed:
outcome:            # success | partial | abandoned (at close)
issue: 72           # this repo's issue
related: ["sertantai-legal#141", 68]
summary: >          # at close
decisions:          # at close: [{what, why, result}]
lessons:            # at close: [{title, detail, tag}] (process insights)
bugs:               # where bugs were found: [{pattern, category, module, affected, fix, status}]
---
```

A finding goes in `lessons` or in `bugs`, never both.

**Sections:**
- `# Session: <Title> (STATUS)`
- `## Problem`: why the work is needed, with the numbers.
- `## Todo`: flat ✅/⬜/⏸️ bullets, one line each. Details go in notes. A ⬜ owned by someone else names them, e.g. "(Jason)" or "(legal)".
- `## Dependencies`
- Dated notes sections, e.g. `## Build notes (2026-09-30)`, for findings and decisions with their evidence.

## Lifecycle

- `pending`: planned, not started.
- `active`
- `suspended`: blocked or paused, with the blocker named in Todo.
- `closed`

**At close** (session-close skill):
- Fill `summary`, `decisions` and `lessons` (and `bugs`).
- Every leftover ⬜ moves to the pending/suspended session where it'll be done (mark it `⏸️ (deferred — moved to …)`), or is dropped with a reason.
- Only one session is active at a time.

## Commits

- Commit the session doc **on its own**, with the subject `session: <topic> — <what>`, at each milestone, not only at close.
- Code changes are committed separately.

## Tooling

- `/usr/bin/python3 scripts/maintenance/session_index.py --root . [--archive]` rebuilds `.claude/sessions/sessions.db` from the frontmatter. It's idempotent.
- `--archive` moves sessions closed more than 30 days ago into `archive/`.
- Skills: session-start (open or resume), session-close, session-archive.

## GitHub

- The issue carries the problem and the decisions; the session carries the working.
- Don't post session results as issue comments unless Jason asks.
