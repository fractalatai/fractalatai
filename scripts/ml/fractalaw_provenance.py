"""Enrichment provenance recording for fractalaw scripts (fractalatai #63).

Calls the hub's SQL functions (created by the Rust CLI's `ensure_provenance_schema`):
`fractalaw_start_run` / `fractalaw_record_stage`. The hub stamps each law with the
lat_hash / struct_hash of the rows the stage read. Never raises: a failure is a
warning, never a failed batch.

Pod batch scripts: copy this file alongside them, and pass the commit in
`FRACTALAW_VERSION` (the pod has no git checkout).
"""

import hashlib
import os
import socket
import subprocess
import sys


def version():
    """Commit of the fractalaw checkout (or FRACTALAW_VERSION)."""
    v = os.environ.get("FRACTALAW_VERSION")
    if v:
        return v
    try:
        return subprocess.run(
            ["git", "rev-parse", "--short=12", "HEAD"],
            capture_output=True, text=True, check=True,
            cwd=os.path.dirname(os.path.abspath(__file__)),
        ).stdout.strip()
    except Exception:
        return "script:" + os.path.basename(sys.argv[0])


def prompt_version(text):
    """Content hash of a prompt, so a prompt edit shows up as a new version."""
    return "sha:" + hashlib.sha256(text.encode()).hexdigest()[:12]


def record(conn, laws, family, stage, method, model, model_version=None, prompt_version=None):
    """Record one stage for `laws` in a new run."""
    laws = sorted({l for l in laws if l})
    if not laws:
        return
    try:
        cur = conn.cursor()
        cmd = " ".join("<url>" if "://" in a else a for a in sys.argv)
        cur.execute("SELECT fractalaw_start_run(%s, %s, %s)", (cmd, version(), socket.gethostname()))
        run = cur.fetchone()[0]
        cur.execute(
            "SELECT fractalaw_record_stage(%s, %s, %s, %s, %s, %s, %s, %s)",
            (run, laws, family, stage, method, model, model_version, prompt_version),
        )
        if not conn.autocommit:
            conn.commit()
        print(f"provenance: {family}/{stage} recorded for {len(laws)} laws (run {run})")
    except Exception as e:
        if not conn.autocommit:
            conn.rollback()
        print(f"warning: enrichment provenance not recorded: {e}")
