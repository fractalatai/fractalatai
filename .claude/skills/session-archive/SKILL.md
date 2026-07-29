---
description: Archive old sessions (closed >30 days) and rebuild the SQLite session index. Run periodically or after closing sessions.
---

# Skill: Session Archive

## When This Applies

When archiving old sessions and rebuilding the SQLite index. Run this periodically (e.g., monthly) or after closing a batch of sessions.

## The Script

`scripts/maintenance/session_index.py` builds a SQLite index from session YAML frontmatter and optionally archives old sessions.

### Flags

| Flag | Description |
|------|-------------|
| `--root .` | Repository root (required) |
| `--archive` | Archive sessions closed >30 days via `git mv` |
| `--archive-days N` | Override the 30-day default |
| `--db PATH` | Override SQLite path (default: `.claude/sessions/sessions.db`) |

## Usage

```bash
# Index + archive (standard run)
/usr/bin/python3 scripts/maintenance/session_index.py --root . --archive

# Index only (no archival)
/usr/bin/python3 scripts/maintenance/session_index.py --root .

# Custom archive threshold (e.g., 14 days)
/usr/bin/python3 scripts/maintenance/session_index.py --root . --archive --archive-days 14

# Query the index
sqlite3 .claude/sessions/sessions.db "SELECT id, title, status FROM sessions WHERE status = 'closed' ORDER BY closed DESC LIMIT 10"
```

## How It Works

1. Scans both active and archive directories for session markdown files
2. Parses YAML frontmatter from each file
3. Drops and recreates all 6 SQLite tables (idempotent)
4. Inserts session metadata, decisions, lessons, metrics, artifacts, and dependencies
5. If `--archive` is set, moves sessions closed >30 days (or `--archive-days N`) to `.claude/sessions/archive/` via `git mv`

### Database Tables

| Table | Content |
|-------|---------|
| `sessions` | Core metadata: id, title, status, opened, closed, outcome, summary |
| `decisions` | What/why/result decisions from frontmatter |
| `lessons` | Title, detail, and tag from lessons learned |
| `metrics` | Session metrics |
| `artifacts` | Files and artifacts produced |
| `dependencies` | Session dependencies |

### Archive Behavior

- Sessions closed >30 days move to `.claude/sessions/archive/` (flat, no subdirectory mirroring)
- Uses `git mv` to preserve history (`git log --follow` recovers full content)
- SQLite retains complete frontmatter for discovery even after archival

## Session Directory Structure

```
.claude/sessions/
├── archive/           # sessions closed >30 days (flat)
├── cascade/           # active/recent cascade sessions
├── compliance/        # compliance sessions
├── fitness/           # fitness sessions
├── jsp/               # JSP sessions
├── parsing/           # batch parse operational sessions
├── store/             # store sessions
├── sessions.db        # SQLite index (all sessions)
└── *.md               # root-level sessions
```

## YAML Frontmatter Format

Sessions must use this format for the indexer to parse them:

```yaml
---
session: "Title here"
status: closed
opened: 2026-07-29
closed: 2026-07-29
outcome: success

summary: >
  Multi-line summary here.

decisions:
  - what: "Quote values containing colons"
    why: "YAML treats bare colons as key:value separators"
    result: "Wrap in quotes"

lessons:
  - title: "Quote values containing colons"
    detail: "Same rule applies"
    tag: operations
---
```

## Common YAML Pitfalls

These break the indexer and must be avoided:

- **Bare colons in values**: `detail: "failed to fill whole buffer" means...` -- the colon after "buffer" starts a new YAML key. Must quote the entire value.
- **Bare quotes in values**: `title: "Any person" in legislation...` -- the unmatched quote breaks parsing. Wrap the whole value in quotes.
- **Fix**: Always wrap values containing colons or quotes in double quotes. Use `>` or `|` block scalars for long text.

## Notes

- Always run with `/usr/bin/python3` (not bare `python3`, which may hit brew's copy)
- The index is idempotent -- drops and recreates all tables on each run, so safe to re-run at any time
- Archived sessions remain fully discoverable via SQLite queries
- Use `git log --follow .claude/sessions/archive/<file>.md` to see pre-archive history
