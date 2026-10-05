#!/usr/bin/python3
"""Label the SLM training sample with the definitive prompt (drrp-v1.1), one model, per provision.

Reads data/training/drrp-v1.1/sample.csv (scripts/ml/sample_drrp_training.py) and writes raw responses to
Postgres drrp_training_labels_raw. Never writes provision_actors.

Versions: `prompt_version` is the prompt rules (drrp_prompt.PROMPT_VERSION); `dict_version` is the actor
dictionary the prompt carried (registered in drrp_dictionary_versions with each label's triggers/patterns).
A dictionary change does NOT relabel everything. A provision labelled under an older dictionary is only
stale (relabelled) if:
  - its response has an OTHER: label or a label no longer in the dictionary (renamed/removed), or
  - its text (with stem) mentions a trigger/pattern of a label added since its dictionary.
Errors are retried on the next run.

Dictionary gaps are reported, never silently absorbed (Jason, 2026-10-05: jump on dictionary diffs):
  - every report lists the OTHER: labels with example provisions;
  - a bulk run (not --pilot) pauses when one new OTHER: actor appears --gate times (default 3). Fix the
    dictionary, then re-run: the stale check relabels only what the fix affects.
    Accepted-unlabelled actors go in data/training/drrp-v1.1/accepted_other.txt (one per line).

  label_drrp_training.py --pilot 200 --dry-run   # which provisions, no calls
  label_drrp_training.py --pilot 200             # pilot: proportional per stratum, then a report
  label_drrp_training.py --family "FIRE,NUCLEAR" --pilot 50   # a family group's pilot (substring match)
  label_drrp_training.py                         # the whole sample
  label_drrp_training.py --report                # report on what's labelled so far
"""

import argparse
import collections
import csv
import hashlib
import json
import os
import random
import re
import sys
import threading
from concurrent.futures import ThreadPoolExecutor, as_completed

import duckdb
import psycopg

ROOT = "/var/home/jason/fractalaw"
sys.path.insert(0, os.path.join(ROOT, "scripts/benchmarks/gold_v2"))
from common import MODELS, PG, PROMPT_VERSION, _post, ancestors, applying, applying_index, gemini_schema, references, system_prompt, user_prompt  # noqa: E402

from drrp_prompt import dictionary_entries, dictionary_version  # noqa: E402

SAMPLE = os.path.join(ROOT, "data/training/drrp-v1.1/sample.csv")
ACCEPTED_OTHER = os.path.join(ROOT, "data/training/drrp-v1.1/accepted_other.txt")
DUCK = os.path.join(ROOT, "data/fractalaw.duckdb")
API = "https://generativelanguage.googleapis.com/v1beta"
# Gemini 3.8 Flash standard rates through 2026-12-31, USD per M tokens (thinking billed as output)
PRICE = {"input": 0.75, "cached": 0.075, "output": 3.75}


def create_cache(model: str, system: str) -> str:
    r = _post(f"{API}/cachedContents?key={os.environ['GEMINI_API_KEY']}",
              {"model": f"models/{model}", "systemInstruction": {"parts": [{"text": system}]}, "ttl": "7200s"}, {})
    return r["name"]


def delete_cache(name: str) -> None:
    import urllib.request
    req = urllib.request.Request(f"{API}/{name}?key={os.environ['GEMINI_API_KEY']}", method="DELETE")
    try:
        urllib.request.urlopen(req, timeout=30)
    except Exception as e:  # expires anyway (ttl)
        print(f"cache delete failed ({e}); it expires on its TTL")


def call(model: str, cache: str | None, system: str, user: str) -> dict:
    body = {"contents": [{"role": "user", "parts": [{"text": user}]}],
            "generationConfig": {"temperature": 0, "responseMimeType": "application/json", "responseSchema": gemini_schema()}}
    if cache:
        body["cachedContent"] = cache
    else:
        body["systemInstruction"] = {"parts": [{"text": system}]}
    r = _post(f"{API}/models/{model}:generateContent?key={os.environ['GEMINI_API_KEY']}", body, {})
    out = json.loads(r["candidates"][0]["content"]["parts"][0]["text"])
    out["_usage"] = r.get("usageMetadata")
    return out


def pick_pilot(rows: list[dict], n: int, seed: int) -> list[dict]:
    """Proportional per stratum (at least max(2, n // 20) each), deterministic."""
    by = collections.defaultdict(list)
    for r in rows:
        by[r["stratum"]].append(r)
    rnd = random.Random(seed)
    out = []
    for s, v in sorted(by.items()):
        v = sorted(v, key=lambda r: r["section_id"])
        rnd.shuffle(v)
        out += v[: max(2, n // 20, round(n * len(v) / len(rows)))]
    return out


def accepted_other() -> set[str]:
    try:
        return {line.split("\t")[0].strip().lower() for line in open(ACCEPTED_OTHER) if line.strip() and not line.startswith("#")}
    except OSError:
        return set()


def other_key(label: str) -> str:
    return re.sub(r"^OTHER:\s*", "", label).strip().lower()


def matcher(entries: dict[str, dict]):
    """One regex per label from its patterns and triggers (word-bounded, case-insensitive)."""
    out = {}
    for label, e in entries.items():
        alts = [p for p in e["patterns"]] + [r"\b" + re.escape(t) + r"\b" for t in e["triggers"] if len(t) > 2]
        if alts:
            try:
                out[label] = re.compile("|".join(f"(?:{a})" for a in alts), re.I)
            except re.error:
                out[label] = re.compile("|".join(r"\b" + re.escape(t) + r"\b" for t in e["triggers"]), re.I)
    return out


def register_dictionary(conn, version: str, entries: dict) -> None:
    conn.execute("INSERT INTO drrp_dictionary_versions (dict_version, labels) VALUES (%s, %s) ON CONFLICT DO NOTHING",
                 (version, json.dumps(entries)))
    conn.commit()


def stale_reason(resp: dict, ctx: str, current: set[str], added: dict) -> str | None:
    for a in resp.get("actors", []):
        if a["label"].startswith("OTHER"):
            return f"OTHER ({a['label']})"
        if a["label"] not in current:
            return f"label gone ({a['label']})"
    for label, rx in added.items():
        if rx.search(ctx):
            return f"new label {label}"
    return None


def report(conn, model: str, ids: set[str] | None, entries: dict) -> None:
    rows = conn.execute(
        "SELECT DISTINCT ON (section_id) section_id, stratum, split, response, error, usage, dict_version "
        "FROM drrp_training_labels_raw WHERE model = %s AND prompt_version = %s "
        "ORDER BY section_id, (error IS NULL) DESC, created_at DESC", (model, PROMPT_VERSION)).fetchall()
    if ids is not None:
        rows = [r for r in rows if r[0] in ids]
    ok = [r for r in rows if r[3] is not None]
    tok = collections.Counter()
    for r in ok:
        u = r[5] or {}
        tok["prompt"] += u.get("promptTokenCount", 0)
        tok["cached"] += u.get("cachedContentTokenCount", 0)
        tok["output"] += u.get("candidatesTokenCount", 0) + u.get("thoughtsTokenCount", 0)
        tok["thoughts"] += u.get("thoughtsTokenCount", 0)
    n = max(len(ok), 1)
    cost = ((tok["prompt"] - tok["cached"]) * PRICE["input"] + tok["cached"] * PRICE["cached"] + tok["output"] * PRICE["output"]) / 1e6
    dicts = collections.Counter(r[6] for r in ok)
    print(f"\n{len(ok)} labelled, {len(rows) - len(ok)} errors ({model}, {PROMPT_VERSION}; dictionaries {dict(dicts)})")
    print(f"tokens per provision: in {tok['prompt'] / n:,.0f} (cached {tok['cached'] / n:,.0f}), "
          f"out {tok['output'] / n:,.0f} (thinking {tok['thoughts'] / n:,.0f}); cost ${cost:.2f} (${cost / n * 1000:.2f} per 1,000)")
    pos, rel, purpose = collections.Counter(), collections.Counter(), collections.Counter()
    by_stratum = collections.defaultdict(collections.Counter)
    others, unknown = collections.defaultdict(list), collections.defaultdict(list)
    inferred = 0
    for sid, stratum, _, resp, _, _, _ in ok:
        rel[resp["relation"]] += 1
        purpose[resp["purpose"]] += 1
        for a in resp["actors"]:
            pos[a["position"]] += 1
            by_stratum[stratum][a["position"]] += 1
            inferred += bool(a.get("inferred"))
            if a["label"].startswith("OTHER"):
                others[other_key(a["label"])].append(sid)
            elif a["label"] not in entries:
                unknown[a["label"]].append(sid)
        if resp["relation"] == "yes" and not [a for a in resp["actors"] if a["position"] == "active"]:
            by_stratum[stratum]["(holder unknown)"] += 1
    print(f"relation: {dict(rel)}; actors: {sum(pos.values())} {dict(pos)}; inferred {inferred}")
    print("per stratum (provisions → actor positions):")
    strata = collections.Counter(r[1] for r in ok)
    for s, c in sorted(by_stratum.items()):
        print(f"  {s:8s} {strata[s]:4d} → " + ", ".join(f"{k} {v}" for k, v in c.most_common()))
    print("purpose: " + ", ".join(f"{k} {v}" for k, v in purpose.most_common()))
    total = sum(strata.values()) or 1
    for p in ("beneficiary", "counterparty"):
        print(f"{p}: {pos[p] / total:.2f} per provision")
    acc = accepted_other()
    print(f"\nDICTIONARY GAPS: {len(others)} OTHER actors ({sum(map(len, others.values()))} uses), "
          f"{len(unknown)} labels not in the dictionary")
    for k, v in sorted(others.items(), key=lambda kv: -len(kv[1])):
        print(f"  OTHER {k!r} ×{len(v)}{' (accepted)' if k in acc else ''}: {', '.join(v[:3])}")
    for k, v in sorted(unknown.items(), key=lambda kv: -len(kv[1])):
        print(f"  NOT IN DICTIONARY {k!r} ×{len(v)}: {', '.join(v[:3])}")
    if others or unknown:
        print("  → actor-drift skill (llm_gaps.py): add/accept each, then re-run; only affected provisions relabel")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--sample", default=SAMPLE)
    ap.add_argument("--pilot", type=int, default=0, help="label a proportional pilot of N provisions")
    ap.add_argument("--family", help="only laws whose DuckDB family contains one of these (comma-separated, case-insensitive)")
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--seed", type=int, default=60)
    ap.add_argument("--workers", type=int, default=8)
    ap.add_argument("--gate", type=int, default=3, help="bulk runs pause when one new OTHER actor appears N times (0: off)")
    ap.add_argument("--no-cache", action="store_true", help="send the system prompt with every call")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--report", action="store_true", help="report only")
    args = ap.parse_args()
    model = MODELS["gemini"]
    entries = dictionary_entries()
    dict_version = dictionary_version()

    rows = list(csv.DictReader(open(args.sample)))
    if args.family:
        fam = dict(duckdb.connect(DUCK, read_only=True).execute("SELECT name, coalesce(family, '') FROM legislation").fetchall())
        wanted = [f.strip().lower() for f in args.family.split(",") if f.strip()]
        rows = [r for r in rows if any(w in fam.get(r["law_name"], "").lower() for w in wanted)]
    if args.pilot:
        rows = pick_pilot(rows, args.pilot, args.seed)
    ids = {r["section_id"] for r in rows}

    with psycopg.connect(PG) as conn:
        register_dictionary(conn, dict_version, entries)
        if args.report:
            report(conn, model, ids, entries)
            return
        laws = sorted({r["law_name"] for r in rows})
        law_rows = conn.execute("SELECT section_id, text, part FROM legislation_text WHERE law_name = ANY(%s)", (laws,)).fetchall()
        prior = conn.execute(
            "SELECT DISTINCT ON (r.section_id, r.text_md5) r.section_id, r.text_md5, r.dict_version, r.response, v.labels "
            "FROM drrp_training_labels_raw r LEFT JOIN drrp_dictionary_versions v USING (dict_version) "
            "WHERE r.model = %s AND r.prompt_version = %s AND r.error IS NULL AND r.section_id = ANY(%s) "
            "ORDER BY r.section_id, r.text_md5, r.created_at DESC", (model, PROMPT_VERSION, sorted(ids))).fetchall()
    texts = {sid: t for sid, t, _ in law_rows}
    parts = {sid: p for sid, _, p in law_rows}
    apps = applying_index(texts, parts)
    current = set(entries)
    match_all = matcher(entries)
    prior_by = {(sid, md5): (dv, resp, labels) for sid, md5, dv, resp, labels in prior}

    jobs, changed, reasons = [], 0, collections.Counter()
    for r in rows:
        sid, text = r["section_id"], texts.get(r["section_id"])
        if not text:
            continue
        md5 = hashlib.md5(text.encode()).hexdigest()
        changed += md5 != r["text_md5"]  # text changed since sampling: label the current text
        stems = [(a, texts[a]) for a in ancestors(sid) if texts.get(a)]
        if (sid, md5) in prior_by:
            dv, resp, labels = prior_by[(sid, md5)]
            if dv == dict_version:
                continue
            added = {lab: match_all[lab] for lab in current - set(labels or {}) if lab in match_all} if labels else match_all
            why = stale_reason(resp, " ".join([t for _, t in stems] + [text]), current, added)
            if not why:
                continue
            reasons[why.split(" (")[0].split(" ")[0] if not why.startswith("new label") else "new label"] += 1
        jobs.append((r, md5, user_prompt(sid, text, stems, references(sid, text, texts), applying(sid, parts, apps))))
    if args.limit:
        jobs = jobs[: args.limit]
    print(f"{model} {PROMPT_VERSION} dict {dict_version}: {len(jobs)} to label of {len(rows)} "
          f"({changed} texts changed since sampling; relabel reasons {dict(reasons)}); "
          f"strata {dict(collections.Counter(r['stratum'] for r, _, _ in jobs))}")
    if args.dry_run or not jobs:
        if not jobs:
            with psycopg.connect(PG) as conn:
                report(conn, model, ids, entries)
        return

    system = system_prompt()
    cache = None if args.no_cache else create_cache(model, system)
    print(f"context cache: {cache or 'off'}")
    gate = args.gate if not args.pilot else 0
    acc = accepted_other()
    seen_other: collections.Counter = collections.Counter()
    stop = threading.Event()

    def run(job):
        if stop.is_set():
            return job, None, "skipped"
        r, md5, prompt = job
        try:
            return job, call(model, cache, system, prompt), None
        except Exception as e:  # recorded, retried on the next run
            return job, None, str(e)[:500]

    ok = err = skipped = 0
    try:
        with ThreadPoolExecutor(args.workers) as pool, psycopg.connect(PG) as conn:
            for fut in as_completed([pool.submit(run, j) for j in jobs]):
                (r, md5, _), resp, error = fut.result()
                if error == "skipped":
                    skipped += 1
                    continue
                usage = resp.pop("_usage", None) if resp else None
                conn.execute(
                    "INSERT INTO drrp_training_labels_raw (section_id, law_name, text_md5, model, prompt_version, dict_version, "
                    "split, stratum, response, error, usage) VALUES (%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s) "
                    "ON CONFLICT (section_id, text_md5, model, prompt_version, dict_version) DO UPDATE SET "
                    "response = EXCLUDED.response, error = EXCLUDED.error, usage = EXCLUDED.usage, created_at = now()",
                    (r["section_id"], r["law_name"], md5, model, PROMPT_VERSION, dict_version, r["split"], r["stratum"],
                     json.dumps(resp) if resp else None, error, json.dumps(usage) if usage else None))
                conn.commit()
                ok, err = ok + (error is None), err + (error is not None)
                for a in (resp or {}).get("actors", []):
                    if a["label"].startswith("OTHER") and other_key(a["label"]) not in acc:
                        seen_other[other_key(a["label"])] += 1
                        if gate and seen_other[other_key(a["label"])] >= gate and not stop.is_set():
                            stop.set()
                            print(f"  GATE: new actor {a['label']!r} seen {gate}× — pausing; fix the dictionary "
                                  f"(or accept it in {ACCEPTED_OTHER}) and re-run", flush=True)
                if (ok + err) % 25 == 0:
                    print(f"  {ok + err}/{len(jobs)} ({err} errors)", flush=True)
    finally:
        if cache:
            delete_cache(cache)
    print(f"done: {ok} labelled, {err} errors, {skipped} skipped by the gate")
    with psycopg.connect(PG) as conn:
        report(conn, model, ids, entries)
    if stop.is_set():
        sys.exit(3)


if __name__ == "__main__":
    main()
