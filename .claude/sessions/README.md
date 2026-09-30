# Session docs

This practice was adopted from sertantai-legal on 2026-09-30. The good example is `2026-09-27-issue-166.md` in legal's `.claude/sessions/`.

## Scope: one issue or one deliverable per session

- **Name:** `<topic>/YYYY-MM-DD-issue-NNN.md` or `<topic>/YYYY-MM-DD-<deliverable>.md`. Older docs use `MM-DD-YY-<name>.md` and stay as they are.
- **The split rule:** when something that comes up mid-session gets its own GitHub issue, or would need its own Todo list, it gets its own session.
  - Open a stub with frontmatter, Problem and Todo.
  - Add a one-line pointer in the parent's Todo: `⬜ → benchmarks/2026-09-30-gold-v2.md`.
  - Never append a second project to a running doc.
- **Size:** ~50–150 lines is healthy. Past ~200 lines, or with more than a handful of dated sections on different topics, split.

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
- Every leftover ⬜ either moves to a named follow-up session or issue (linked), or is explicitly dropped with a reason.

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
