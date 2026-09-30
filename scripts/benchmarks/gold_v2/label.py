#!/usr/bin/python3
"""Label every substantive provision of the given laws with one model (gold v2).

Resumable: a provision already labelled for (text_md5, model, prompt_version)
is skipped. Raw responses go to gold_v2_raw.

  /usr/bin/python3 scripts/benchmarks/gold_v2/label.py --model gemini --laws UK_asp_2005_13 [--limit 10] [--workers 4]
"""

import argparse
import hashlib
import json
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from common import CALLERS, PROMPT_VERSION, MODELS, ancestors, connect, references, system_prompt, user_prompt  # noqa: E402


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", choices=sorted(CALLERS), required=True)
    ap.add_argument("--laws", required=True)
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--workers", type=int, default=4)
    args = ap.parse_args()
    laws = [x.strip() for x in args.laws.split(",") if x.strip()]
    model = MODELS[args.model]
    system = system_prompt()
    call = CALLERS[args.model]

    with connect() as conn:
        rows = conn.execute(
            "SELECT section_id, law_name, text FROM legislation_text "
            "WHERE law_name = ANY(%s) AND scope = 'substantive' AND text IS NOT NULL ORDER BY law_name, sort_key",
            (laws,),
        ).fetchall()
        texts = dict(conn.execute(
            "SELECT section_id, text FROM legislation_text WHERE law_name = ANY(%s)", (laws,)).fetchall())
        done = {r[0] for r in conn.execute(
            "SELECT section_id || '|' || text_md5 FROM gold_v2_raw WHERE model = %s AND prompt_version = %s AND error IS NULL",
            (model, PROMPT_VERSION)).fetchall()}

    jobs = []
    for sid, law, text in rows:
        md5 = hashlib.md5(text.encode()).hexdigest()
        if f"{sid}|{md5}" in done:
            continue
        stems = [(a, texts[a]) for a in ancestors(sid) if texts.get(a)]
        jobs.append((sid, law, md5, user_prompt(sid, text, stems, references(sid, text, texts))))
    if args.limit:
        jobs = jobs[: args.limit]
    print(f"{model}: {len(jobs)} provisions to label ({len(rows)} substantive, {len(rows) - len(jobs)} already done or skipped)")

    def run(job):
        sid, law, md5, prompt = job
        try:
            return job, call(system, prompt), None
        except Exception as e:  # recorded, retried on the next run
            return job, None, str(e)[:500]

    ok = err = 0
    with ThreadPoolExecutor(args.workers) as pool, connect() as conn:
        for fut in as_completed([pool.submit(run, j) for j in jobs]):
            (sid, law, md5, _), resp, error = fut.result()
            usage = resp.pop("_usage", None) if resp else None
            conn.execute(
                "INSERT INTO gold_v2_raw (section_id, law_name, text_md5, model, prompt_version, response, error, usage) "
                "VALUES (%s,%s,%s,%s,%s,%s,%s,%s) ON CONFLICT (section_id, text_md5, model, prompt_version) "
                "DO UPDATE SET response = EXCLUDED.response, error = EXCLUDED.error, usage = EXCLUDED.usage, created_at = now()",
                (sid, law, md5, model, PROMPT_VERSION, json.dumps(resp) if resp else None, error,
                 json.dumps(usage) if usage else None),
            )
            conn.commit()
            ok, err = ok + (error is None), err + (error is not None)
            if (ok + err) % 25 == 0:
                print(f"  {ok + err}/{len(jobs)} ({err} errors)", flush=True)
    print(f"done: {ok} labelled, {err} errors")


if __name__ == "__main__":
    main()
