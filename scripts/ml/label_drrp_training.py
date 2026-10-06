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
  label_drrp_training.py --family "FIRE,NUCLEAR" --pilot 50   # a family group's pilot (substring match; "(none)" = no family)
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
from common import MODELS, PG, PROMPT_VERSION, _post, ancestors, call_openai, applying, applying_index, gemini_schema, references, system_prompt, user_prompt  # noqa: E402

from drrp_prompt import dictionary_entries, dictionary_version  # noqa: E402

SAMPLE = os.path.join(ROOT, "data/training/drrp-v1.1/sample.csv")
ACCEPTED_OTHER = os.path.join(ROOT, "data/training/drrp-v1.1/accepted_other.txt")
DUCK = os.path.join(ROOT, "data/fractalaw.duckdb")
API = "https://generativelanguage.googleapis.com/v1beta"
# USD per M tokens (thinking/reasoning billed as output). Gemini 3.8 Flash standard rates through 2026-12-31;
# GPT-5.5 standard (2026-10-05)
PRICES = {"gemini": {"input": 0.75, "cached": 0.075, "output": 3.75},
          "openai": {"input": 5.00, "cached": 0.50, "output": 30.00},
          "gpt-5.4-mini": {"input": 0.75, "cached": 0.075, "output": 4.50}}


def usage_tokens(u: dict) -> tuple[int, int, int, int]:
    """(prompt, cached, output incl. thinking, thinking) from a Gemini usageMetadata or an OpenAI usage."""
    u = u or {}
    if "input_tokens" in u:  # OpenAI Responses API
        return (u.get("input_tokens", 0), (u.get("input_tokens_details") or {}).get("cached_tokens", 0),
                u.get("output_tokens", 0), (u.get("output_tokens_details") or {}).get("reasoning_tokens", 0))
    return (u.get("promptTokenCount", 0), u.get("cachedContentTokenCount", 0),
            u.get("candidatesTokenCount", 0) + u.get("thoughtsTokenCount", 0), u.get("thoughtsTokenCount", 0))


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


def stale_reason(resp: dict, ctx: str, current: set[str], added: dict, changed: set[str] = frozenset()) -> str | None:
    for a in resp.get("actors", []):
        if a["label"].startswith("OTHER"):
            return f"OTHER ({a['label']})"
        if a["label"] not in current:
            return f"label gone ({a['label']})"
        if a["label"] in changed:
            return f"changed label ({a['label']})"
    for label, rx in added.items():
        if rx.search(ctx):
            return f"new label {label}"
    return None


# Rules added in drrp-v1.2 (Jason, 2026-10-05; DRRP-CLASSIFICATION.md special cases): a v1.1 label is carried
# forward unless one of these could change it.
_MEMBER_STATE = re.compile(r"\bMember States?\b|\bthird countr", re.I)
_TRANSITIONAL = re.compile(r"\btransitional\b", re.I)
_HEADLESS = re.compile(r"\b(?:shall|must|may)\b[^.;]{0,120}(?:[—–:]|\s-)\s*$", re.I)


def v12_affected(ctx: str, text: str, resp: dict) -> str | None:
    actors = resp.get("actors") or []
    if _MEMBER_STATE.search(ctx) or any(a["label"] == "EU: Member State" for a in actors):
        return "member state/place"
    if resp.get("purpose") == "Transitional Arrangement" or _TRANSITIONAL.search(ctx):
        return "transitional"
    if _HEADLESS.search(text.rstrip()) and (resp.get("relation") == "no" or resp.get("purpose") == "Procedure+Detail"):
        return "headless stem"
    passive = resp.get("relation") == "yes" and not any(a["position"] == "active" for a in actors)
    if passive and any(a["position"] == "counterparty" and not a.get("act") for a in actors):
        return "passive counterparty act"
    return None


# Rules added in drrp-v1.3 (Jason, 2026-10-05, from the batch 1 referee). Class-definition items have no reliable
# text signal; the referee catches them.
_TRIGGER = re.compile(r"having regard to|\bwhere\b[^.;]{0,80}\b(?:risks?|danger)|in (?:the )?light of|taking (?:into )?account", re.I)
_COMMENCE = re.compile(r"on such days? as|may by order appoint|appointed day", re.I)
_LAYING = re.compile(r"\blai?(?:d|ys?|ying)\b[^.;]{0,80}\bbefore\b[^.;]{0,30}\b(?:Parliament|Assembly|House)", re.I)
_ENFORCING = re.compile(r"(?:shall be|is|are)\s+(?:responsible\s+as\s+)?the\s+enforcing\s+authorit|\bfunctions\s+of\s+(?:the|a)\s+[^.;]{0,40}(?:committee|board|council)", re.I)
_MONEY = re.compile(r"money provided by Parliament", re.I)
_DISAPPLY = re.compile(r"shall not apply[^.;]{0,80}\b(?:until|before|after)\b", re.I)


def v13_affected(ctx: str, text: str, resp: dict) -> str | None:
    actors = resp.get("actors") or []
    if any(a["position"] == "beneficiary" for a in actors) and _TRIGGER.search(ctx):
        return "trigger-condition beneficiary"
    if _COMMENCE.search(ctx) or resp.get("purpose") == "Enactment+Citation+Commencement":
        return "commencement power"
    if _LAYING.search(ctx):
        return "laying before Parliament"
    if _ENFORCING.search(text):
        return "enforcing authority / functions"
    if _MONEY.search(ctx):
        return "money provided by Parliament"
    if _DISAPPLY.search(ctx) or resp.get("purpose") == "Transitional Arrangement":
        return "time-limited disapplication"
    return None


# The rules a label must be checked against when carried forward INTO each prompt version
RULESETS = {"drrp-v1.2-2026-10-05": v12_affected, "drrp-v1.3-2026-10-05": v13_affected}


def carry_forward(conn, model: str, carry_from: str, rows: list[dict], texts: dict[str, str]) -> collections.Counter:
    """Copy each provision's latest `carry_from` label to PROMPT_VERSION unless a rule new in PROMPT_VERSION could change it."""
    affected = RULESETS[PROMPT_VERSION]
    have = {r[0] for r in conn.execute(
        "SELECT section_id || '|' || text_md5 FROM drrp_training_labels_raw WHERE model = %s AND prompt_version = %s",
        (model, PROMPT_VERSION)).fetchall()}
    old = conn.execute(
        "SELECT DISTINCT ON (section_id, text_md5) section_id, text_md5, law_name, split, stratum, dict_version, response "
        "FROM drrp_training_labels_raw WHERE model = %s AND prompt_version = %s AND error IS NULL AND section_id = ANY(%s) "
        "ORDER BY section_id, text_md5, created_at DESC", (model, carry_from, [r["section_id"] for r in rows])).fetchall()
    out = collections.Counter()
    for sid, md5, law, split, stratum, dv, resp in old:
        text = texts.get(sid)
        if not text or hashlib.md5(text.encode()).hexdigest() != md5 or f"{sid}|{md5}" in have:
            continue
        ctx = " ".join([texts.get(a) or "" for a in ancestors(sid)] + [text])
        why = affected(ctx, text, resp)
        if why:
            out[why] += 1
            continue
        conn.execute(
            "INSERT INTO drrp_training_labels_raw (section_id, law_name, text_md5, model, prompt_version, dict_version, split, "
            "stratum, response, usage) VALUES (%s,%s,%s,%s,%s,%s,%s,%s,%s,%s) ON CONFLICT DO NOTHING",
            (sid, law, md5, model, PROMPT_VERSION, dv, split, stratum, json.dumps(resp), json.dumps({"carried_from": carry_from})))
        out["carried"] += 1
    conn.commit()
    return out


def report(conn, model: str, ids: set[str] | None, entries: dict, price: dict | None = None) -> None:
    price = price or PRICES["gemini"]
    rows = conn.execute(
        "SELECT DISTINCT ON (section_id) section_id, stratum, split, response, error, usage, dict_version "
        "FROM drrp_training_labels_raw WHERE model = %s AND prompt_version = %s "
        "ORDER BY section_id, (error IS NULL) DESC, created_at DESC", (model, PROMPT_VERSION)).fetchall()
    if ids is not None:
        rows = [r for r in rows if r[0] in ids]
    ok = [r for r in rows if r[3] is not None]
    tok = collections.Counter()
    for r in ok:
        p, c, o, t = usage_tokens(r[5])
        tok["prompt"] += p
        tok["cached"] += c
        tok["output"] += o
        tok["thoughts"] += t
    n = max(len(ok), 1)
    cost = ((tok["prompt"] - tok["cached"]) * price["input"] + tok["cached"] * price["cached"] + tok["output"] * price["output"]) / 1e6
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
    ap.add_argument("--no-cache", action="store_true", help="send the system prompt with every call (Gemini)")
    ap.add_argument("--model", choices=["gemini", "openai"], default="gemini",
                    help="gemini (gemini-3.8-flash) or openai (gpt-5.5:low; prompt caching is automatic)")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--model-name", help="override the provider's model, e.g. gpt-5.4-mini:low")
    ap.add_argument("--ids", help="file of section_ids to label (one per line), within the sample")
    ap.add_argument("--carry-from", help="carry labels from this prompt version unless a v1.2 rule could change them")
    ap.add_argument("--report", action="store_true", help="report only")
    args = ap.parse_args()
    if args.model_name:
        MODELS[args.model] = args.model_name
    model = MODELS[args.model]
    price = PRICES.get(model.split(":")[0], PRICES[args.model])
    entries = dictionary_entries()
    dict_version = dictionary_version()

    rows = list(csv.DictReader(open(args.sample)))
    if args.family:
        fam = dict(duckdb.connect(DUCK, read_only=True).execute("SELECT name, coalesce(family, '') FROM legislation").fetchall())
        wanted = [f.strip().lower() for f in args.family.split(",") if f.strip()]
        # "(none)" selects laws with no family in DuckDB (empty or missing)
        rows = [r for r in rows if any((w == "(none)" and not fam.get(r["law_name"], "").strip())
                                       or (w != "(none)" and w in fam.get(r["law_name"], "").lower()) for w in wanted)]
    if args.ids:
        wanted_ids = {line.strip() for line in open(args.ids) if line.strip()}
        rows = [r for r in rows if r["section_id"] in wanted_ids]
    if args.pilot:
        rows = pick_pilot(rows, args.pilot, args.seed)
    ids = {r["section_id"] for r in rows}

    with psycopg.connect(PG) as conn:
        register_dictionary(conn, dict_version, entries)
        if args.report:
            report(conn, model, ids, entries, price)
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
    if args.carry_from and not args.dry_run:
        with psycopg.connect(PG) as conn:
            print(f"carry-forward from {args.carry_from}: {dict(carry_forward(conn, model, args.carry_from, rows, texts))}")
        with psycopg.connect(PG) as conn:
            prior = conn.execute(
                "SELECT DISTINCT ON (r.section_id, r.text_md5) r.section_id, r.text_md5, r.dict_version, r.response, v.labels "
                "FROM drrp_training_labels_raw r LEFT JOIN drrp_dictionary_versions v USING (dict_version) "
                "WHERE r.model = %s AND r.prompt_version = %s AND r.error IS NULL AND r.section_id = ANY(%s) "
                "ORDER BY r.section_id, r.text_md5, r.created_at DESC", (model, PROMPT_VERSION, sorted(ids))).fetchall()
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
            # labels whose triggers/patterns changed since: a response using one may have chosen differently
            changed_labels = {lab for lab in current & set(labels or {}) if labels[lab] != entries[lab]}
            added.update({lab: match_all[lab] for lab in changed_labels if lab in match_all})
            why = stale_reason(resp, " ".join([t for _, t in stems] + [text]), current, added, changed_labels)
            if not why:
                continue
            reasons[why.split(" (")[0] if not why.startswith("new label") else "new/changed label text"] += 1
        jobs.append((r, md5, user_prompt(sid, text, stems, references(sid, text, texts), applying(sid, parts, apps))))
    if args.limit:
        jobs = jobs[: args.limit]
    print(f"{model} {PROMPT_VERSION} dict {dict_version}: {len(jobs)} to label of {len(rows)} "
          f"({changed} texts changed since sampling; relabel reasons {dict(reasons)}); "
          f"strata {dict(collections.Counter(r['stratum'] for r, _, _ in jobs))}")
    if args.dry_run or not jobs:
        if not jobs:
            with psycopg.connect(PG) as conn:
                report(conn, model, ids, entries, price)
        return

    system = system_prompt()
    cache = None if (args.no_cache or args.model != "gemini") else create_cache(model, system)
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
            if args.model == "openai":
                return job, call_openai(system, prompt), None
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
                if error and ("insufficient_quota" in error or "no credits remaining" in error) and not stop.is_set():
                    stop.set()  # a billing failure won't clear by retrying: stop now
                    print(f"  STOP: the API account is out of credit ({error[:120]}…). Top up and re-run; failed rows are retried.", flush=True)
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
        report(conn, model, ids, entries, price)
    if stop.is_set():
        sys.exit(3)


if __name__ == "__main__":
    main()
