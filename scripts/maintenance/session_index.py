#!/usr/bin/env python3
"""Build a SQLite index from session doc YAML frontmatter.

Scans .claude/sessions/**/*.md for YAML frontmatter (between --- fences),
parses it, and populates a normalised SQLite database. Idempotent — drops
and recreates all tables on each run.

Usage:
    python3 scripts/maintenance/session_index.py                    # default: .claude/sessions/
    python3 scripts/maintenance/session_index.py --root /path/to/repo
    python3 scripts/maintenance/session_index.py --archive          # also archive old sessions
"""

import argparse
import json
import os
import re
import sqlite3
import subprocess
import sys
from datetime import date, datetime, timedelta
from pathlib import Path

import yaml


SCHEMA = """
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    subdir TEXT,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    outcome TEXT,
    opened TEXT,
    closed TEXT,
    summary TEXT
);

CREATE TABLE decisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    what TEXT NOT NULL,
    why TEXT,
    result TEXT
);

CREATE TABLE lessons (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    title TEXT NOT NULL,
    detail TEXT,
    tag TEXT
);

CREATE TABLE metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    key TEXT NOT NULL,
    value TEXT NOT NULL
);

CREATE TABLE artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    path TEXT NOT NULL
);

CREATE TABLE dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    direction TEXT NOT NULL,
    target TEXT NOT NULL
);

CREATE INDEX idx_sessions_status ON sessions(status);
CREATE INDEX idx_sessions_opened ON sessions(opened);
CREATE INDEX idx_lessons_tag ON lessons(tag);
CREATE INDEX idx_dependencies_direction ON dependencies(direction);
"""


def parse_frontmatter(filepath: Path) -> dict | None:
    """Extract YAML frontmatter from a markdown file."""
    text = filepath.read_text(encoding="utf-8")

    # Match --- fenced YAML at the start of the file
    match = re.match(r"^---\s*\n(.*?)\n---\s*\n", text, re.DOTALL)
    if not match:
        return None

    try:
        return yaml.safe_load(match.group(1))
    except yaml.YAMLError as e:
        print(f"  WARN: YAML parse error in {filepath}: {e}", file=sys.stderr)
        return None


def session_id_from_path(filepath: Path, sessions_dir: Path) -> str:
    """Derive a session ID from the file path relative to sessions dir."""
    rel = filepath.relative_to(sessions_dir)
    return rel.with_suffix("").as_posix()


def index_session(conn: sqlite3.Connection, filepath: Path, sessions_dir: Path):
    """Parse one session file and insert into the database."""
    fm = parse_frontmatter(filepath)
    if fm is None:
        return False

    sid = session_id_from_path(filepath, sessions_dir)
    rel_path = str(filepath.relative_to(sessions_dir.parent.parent))  # relative to repo root

    # Determine subdirectory (cascade, store, etc.)
    parts = filepath.relative_to(sessions_dir).parts
    subdir = parts[0] if len(parts) > 1 else None

    title = fm.get("session", sid)
    status = fm.get("status", "unknown")
    outcome = fm.get("outcome")
    opened = str(fm["opened"]) if fm.get("opened") else None
    closed = str(fm["closed"]) if fm.get("closed") else None
    summary = fm.get("summary", "").strip() if fm.get("summary") else None

    conn.execute(
        "INSERT INTO sessions (id, path, subdir, title, status, outcome, opened, closed, summary) "
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        (sid, rel_path, subdir, title, status, outcome, opened, closed, summary),
    )

    # Decisions
    for d in fm.get("decisions") or []:
        if isinstance(d, dict):
            conn.execute(
                "INSERT INTO decisions (session_id, what, why, result) VALUES (?, ?, ?, ?)",
                (sid, d.get("what", ""), d.get("why"), d.get("result")),
            )

    # Lessons
    for l in fm.get("lessons") or []:
        if isinstance(l, dict):
            conn.execute(
                "INSERT INTO lessons (session_id, title, detail, tag) VALUES (?, ?, ?, ?)",
                (sid, l.get("title", ""), l.get("detail"), l.get("tag")),
            )

    # Metrics — flatten nested dicts to key/JSON-value pairs
    for key, value in (fm.get("metrics") or {}).items():
        if isinstance(value, dict):
            val_str = json.dumps(value)
        else:
            val_str = str(value)
        conn.execute(
            "INSERT INTO metrics (session_id, key, value) VALUES (?, ?, ?)",
            (sid, key, val_str),
        )

    # Artifacts
    for a in fm.get("artifacts") or []:
        if isinstance(a, str):
            conn.execute(
                "INSERT INTO artifacts (session_id, path) VALUES (?, ?)",
                (sid, a),
            )

    # Dependencies
    for target in fm.get("depends_on") or []:
        if isinstance(target, str):
            conn.execute(
                "INSERT INTO dependencies (session_id, direction, target) VALUES (?, ?, ?)",
                (sid, "depends_on", target),
            )
    for target in fm.get("enables") or []:
        if isinstance(target, str):
            conn.execute(
                "INSERT INTO dependencies (session_id, direction, target) VALUES (?, ?, ?)",
                (sid, "enables", target),
            )

    return True


def build_index(sessions_dir: Path, db_path: Path):
    """Scan all session docs and build the SQLite index."""
    conn = sqlite3.connect(str(db_path))

    # Drop and recreate (idempotent rebuild)
    for table in ["dependencies", "artifacts", "metrics", "lessons", "decisions", "sessions"]:
        conn.execute(f"DROP TABLE IF EXISTS {table}")
    conn.executescript(SCHEMA)

    indexed = 0
    skipped = 0
    for filepath in sorted(sessions_dir.rglob("*.md")):
        if index_session(conn, filepath, sessions_dir):
            indexed += 1
        else:
            skipped += 1

    conn.commit()

    # Summary
    counts = {}
    for table in ["sessions", "decisions", "lessons", "metrics", "artifacts", "dependencies"]:
        row = conn.execute(f"SELECT COUNT(*) FROM {table}").fetchone()
        counts[table] = row[0]

    print(f"Indexed {indexed} sessions ({skipped} without frontmatter)")
    for table, count in counts.items():
        print(f"  {table}: {count}")

    conn.close()
    return indexed


def archive_old_sessions(sessions_dir: Path, days: int = 30):
    """Move sessions closed >N days ago to archive/ subdirectory via git mv."""
    cutoff = date.today() - timedelta(days=days)
    archive_dir = sessions_dir / "archive"
    moved = []

    for filepath in sorted(sessions_dir.rglob("*.md")):
        # Skip files already in archive
        if "archive" in filepath.parts:
            continue

        fm = parse_frontmatter(filepath)
        if fm is None:
            continue
        if fm.get("status") != "closed":
            continue

        closed = fm.get("closed")
        if closed is None:
            continue

        closed_date = closed if isinstance(closed, date) else date.fromisoformat(str(closed))
        if closed_date >= cutoff:
            continue

        # Determine target path
        rel = filepath.relative_to(sessions_dir)
        if len(rel.parts) > 1:
            # Has a subdirectory (e.g. cascade/foo.md → archive/cascade/foo.md)
            target_dir = archive_dir / rel.parent
        else:
            # Top-level session → archive/foo.md
            target_dir = archive_dir

        target_dir.mkdir(parents=True, exist_ok=True)
        target = target_dir / filepath.name

        try:
            subprocess.run(
                ["git", "mv", str(filepath), str(target)],
                check=True,
                capture_output=True,
                cwd=sessions_dir.parent.parent,  # repo root
            )
            moved.append((str(rel), str(target.relative_to(sessions_dir))))
        except subprocess.CalledProcessError as e:
            print(f"  WARN: git mv failed for {rel}: {e.stderr.decode()}", file=sys.stderr)

    if moved:
        print(f"\nArchived {len(moved)} sessions (closed before {cutoff}):")
        for src, dst in moved:
            print(f"  {src} → {dst}")
    else:
        print(f"\nNo sessions to archive (cutoff: closed before {cutoff})")

    return moved


def main():
    parser = argparse.ArgumentParser(description="Build SQLite index from session frontmatter")
    parser.add_argument(
        "--root",
        type=Path,
        default=Path("."),
        help="Repository root (default: current directory)",
    )
    parser.add_argument(
        "--db",
        type=Path,
        default=None,
        help="SQLite database path (default: <root>/.claude/sessions/sessions.db)",
    )
    parser.add_argument(
        "--archive",
        action="store_true",
        help="Archive sessions closed >30 days ago via git mv",
    )
    parser.add_argument(
        "--archive-days",
        type=int,
        default=30,
        help="Archive cutoff in days (default: 30)",
    )
    args = parser.parse_args()

    root = args.root.resolve()
    sessions_dir = root / ".claude" / "sessions"
    if not sessions_dir.exists():
        print(f"Sessions directory not found: {sessions_dir}", file=sys.stderr)
        sys.exit(1)

    db_path = args.db or (sessions_dir / "sessions.db")

    print(f"Scanning: {sessions_dir}")
    print(f"Database: {db_path}")

    if args.archive:
        archive_old_sessions(sessions_dir, args.archive_days)

    build_index(sessions_dir, db_path)


if __name__ == "__main__":
    main()
