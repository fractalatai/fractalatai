#!/usr/bin/python3
"""Label the SLM training sample with the definitive prompt (drrp-v1.1), one model, per provision.

Reads data/training/drrp-v1.1/sample.csv (scripts/ml/sample_drrp_training.py) and writes raw responses to
Postgres drrp_training_labels_raw (resumable: done (section_id, text_md5, model, prompt_version) rows are
skipped; errors are retried on the next run). Never writes provision_actors.

The system prompt goes in an explicit Gemini context cache (cachedContents), so each call pays the cached
input rate for it. Prompts are built exactly as gold v2 label.py does: stem + referenced + applying provisions.

  label_drrp_training.py --pilot 200 --dry-run   # which provisions, no calls
  label_drrp_training.py --pilot 200             # pilot: proportional per stratum, then a report
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
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed

import psycopg

ROOT = "/var/home/jason/fractalaw"
sys.path.insert(0, os.path.join(ROOT, "scripts/benchmarks/gold_v2"))
from common import MODELS, PG, PROMPT_VERSION, _post, ancestors, applying, applying_index, gemini_schema, references, system_prompt, user_prompt  # noqa: E402

SAMPLE = os.path.join(ROOT, "data/training/drrp-v1.1/sample.csv")
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
    """Proportional per stratum (at least 10 each), deterministic."""
    by = collections.defaultdict(list)
    for r in rows:
        by[r["stratum"]].append(r)
    rnd = random.Random(seed)
    out = []
    for s, v in sorted(by.items()):
        v = sorted(v, key=lambda r: r["section_id"])
        rnd.shuffle(v)
        out += v[: max(10, round(n * len(v) / len(rows)))]
    return out


def report(conn, model: str, ids: set[str] | None) -> None:
    rows = conn.execute(
        "SELECT section_id, stratum, split, response, error, usage FROM drrp_training_labels_raw "
        "WHERE model = %s AND prompt_version = %s", (model, PROMPT_VERSION)).fetchall()
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
    print(f"\n{len(ok)} labelled, {len(rows) - len(ok)} errors ({model}, {PROMPT_VERSION})")
    print(f"tokens per provision: in {tok['prompt'] / n:,.0f} (cached {tok['cached'] / n:,.0f}), "
          f"out {tok['output'] / n:,.0f} (thinking {tok['thoughts'] / n:,.0f}); cost ${cost:.2f} (${cost / n * 1000:.2f} per 1,000)")
    pos = collections.Counter()
    by_stratum = collections.defaultdict(collections.Counter)
    rel = collections.Counter()
    purpose = collections.Counter()
    other, inferred = 0, 0
    for sid, stratum, _, resp, _, _ in ok:
        rel[resp["relation"]] += 1
        purpose[resp["purpose"]] += 1
        for a in resp["actors"]:
            pos[a["position"]] += 1
            by_stratum[stratum][a["position"]] += 1
            other += a["label"].startswith("OTHER")
            inferred += bool(a.get("inferred"))
        holders = [a for a in resp["actors"] if a["position"] == "active"]
        if resp["relation"] == "yes" and not holders:
            by_stratum[stratum]["(holder unknown)"] += 1
    print(f"relation: {dict(rel)}; actors: {sum(pos.values())} {dict(pos)}; OTHER labels {other}; inferred {inferred}")
    print("per stratum (provisions → actor positions):")
    strata = collections.Counter(r[1] for r in ok)
    for s, c in sorted(by_stratum.items()):
        print(f"  {s:8s} {strata[s]:4d} → " + ", ".join(f"{k} {v}" for k, v in c.most_common()))
    print("purpose: " + ", ".join(f"{k} {v}" for k, v in purpose.most_common()))
    total = sum(strata.values()) or 1
    for p in ("beneficiary", "counterparty"):
        print(f"{p}: {pos[p] / total:.2f} per provision → ~{pos[p] / total * 6121:,.0f} in the full sample")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--sample", default=SAMPLE)
    ap.add_argument("--pilot", type=int, default=0, help="label a proportional pilot of N provisions")
    ap.add_argument("--seed", type=int, default=60)
    ap.add_argument("--workers", type=int, default=8)
    ap.add_argument("--no-cache", action="store_true", help="send the system prompt with every call")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--report", action="store_true", help="report only")
    args = ap.parse_args()
    model = MODELS["gemini"]

    rows = list(csv.DictReader(open(args.sample)))
    if args.pilot:
        rows = pick_pilot(rows, args.pilot, args.seed)
    ids = {r["section_id"] for r in rows}

    with psycopg.connect(PG) as conn:
        if args.report:
            report(conn, model, ids)
            return
        laws = sorted({r["law_name"] for r in rows})
        law_rows = conn.execute("SELECT section_id, text, part FROM legislation_text WHERE law_name = ANY(%s)", (laws,)).fetchall()
        done = {r[0] for r in conn.execute(
            "SELECT section_id || '|' || text_md5 FROM drrp_training_labels_raw "
            "WHERE model = %s AND prompt_version = %s AND error IS NULL", (model, PROMPT_VERSION)).fetchall()}
    texts = {sid: t for sid, t, _ in law_rows}
    parts = {sid: p for sid, _, p in law_rows}
    apps = applying_index(texts, parts)

    jobs, changed = [], 0
    for r in rows:
        sid, text = r["section_id"], texts.get(r["section_id"])
        if not text:
            continue
        md5 = hashlib.md5(text.encode()).hexdigest()
        changed += md5 != r["text_md5"]  # text changed since sampling: label the current text
        if f"{sid}|{md5}" in done:
            continue
        stems = [(a, texts[a]) for a in ancestors(sid) if texts.get(a)]
        jobs.append((r, md5, user_prompt(sid, text, stems, references(sid, text, texts), applying(sid, parts, apps))))
    print(f"{model} {PROMPT_VERSION}: {len(jobs)} to label of {len(rows)} ({len(rows) - len(jobs)} done or missing; "
          f"{changed} texts changed since sampling); strata {dict(collections.Counter(r['stratum'] for r, _, _ in jobs))}")
    if args.dry_run or not jobs:
        if not jobs:
            with psycopg.connect(PG) as conn:
                report(conn, model, ids)
        return

    system = system_prompt()
    cache = None if args.no_cache else create_cache(model, system)
    print(f"context cache: {cache or 'off'}")

    def run(job):
        r, md5, prompt = job
        try:
            return job, call(model, cache, system, prompt), None
        except Exception as e:  # recorded, retried on the next run
            return job, None, str(e)[:500]

    ok = err = 0
    try:
        with ThreadPoolExecutor(args.workers) as pool, psycopg.connect(PG) as conn:
            for fut in as_completed([pool.submit(run, j) for j in jobs]):
                (r, md5, _), resp, error = fut.result()
                usage = resp.pop("_usage", None) if resp else None
                conn.execute(
                    "INSERT INTO drrp_training_labels_raw (section_id, law_name, text_md5, model, prompt_version, split, stratum, "
                    "response, error, usage) VALUES (%s,%s,%s,%s,%s,%s,%s,%s,%s,%s) "
                    "ON CONFLICT (section_id, text_md5, model, prompt_version) DO UPDATE SET response = EXCLUDED.response, "
                    "error = EXCLUDED.error, usage = EXCLUDED.usage, created_at = now()",
                    (r["section_id"], r["law_name"], md5, model, PROMPT_VERSION, r["split"], r["stratum"],
                     json.dumps(resp) if resp else None, error, json.dumps(usage) if usage else None))
                conn.commit()
                ok, err = ok + (error is None), err + (error is not None)
                if (ok + err) % 25 == 0:
                    print(f"  {ok + err}/{len(jobs)} ({err} errors)", flush=True)
    finally:
        if cache:
            delete_cache(cache)
    print(f"done: {ok} labelled, {err} errors")
    with psycopg.connect(PG) as conn:
        report(conn, model, ids)


if __name__ == "__main__":
    main()
