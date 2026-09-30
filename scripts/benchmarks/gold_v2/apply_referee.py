#!/usr/bin/python3
"""Write the Claude referee's decisions on disputed provisions into gold v2 (source 'referee').

Input JSONL, one object per disputed provision:
  {"section_id", "text_md5", "relation", "raw_type", "actors": [{"label", "position", "holds", "inferred"}], "reason"}
A decision whose text_md5 no longer matches the hub text is skipped.

  /usr/bin/python3 scripts/benchmarks/gold_v2/apply_referee.py data/audit/gold_v2_referee_<law>.jsonl
"""

import json
import sys

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from common import PROMPT_VERSION, connect, norm_label  # noqa: E402

POSITIONS = {"active", "counterparty", "beneficiary", "mentioned"}
HOLDS = {"Obligation", "Liberty", "none"}


def main() -> None:
    path = sys.argv[1]
    applied = skipped = 0
    with connect() as conn:
        for line in open(path):
            d = json.loads(line)
            sid = d["section_id"]
            row = conn.execute("SELECT law_name, md5(text) FROM legislation_text WHERE section_id = %s", (sid,)).fetchone()
            if not row or row[1] != d["text_md5"]:
                print(f"  skip {sid}: text changed or missing")
                skipped += 1
                continue
            law = row[0]
            actors = {}
            for a in d["actors"]:
                pos, holds = a["position"], a["holds"] if a["position"] == "active" else "none"
                assert pos in POSITIONS and holds in HOLDS, (sid, a)
                actors[norm_label(a["label"])] = (pos, holds, bool(a.get("inferred")))
            conn.execute("DELETE FROM gold_v2 WHERE section_id = %s", (sid,))
            for label, (pos, holds, inferred) in actors.items():
                conn.execute(
                    "INSERT INTO gold_v2 (section_id, law_name, text_md5, actor_label, position, holds, inferred, source, note, prompt_version) "
                    "VALUES (%s,%s,%s,%s,%s,%s,%s,'referee',%s,%s)",
                    (sid, law, d["text_md5"], label, pos, holds, inferred, d.get("reason"), PROMPT_VERSION))
            conn.execute(
                "INSERT INTO gold_v2_provision (section_id, law_name, text_md5, relation, raw_type, source, note, prompt_version) "
                "VALUES (%s,%s,%s,%s,%s,'referee',%s,%s) ON CONFLICT (section_id) DO UPDATE SET text_md5 = EXCLUDED.text_md5, "
                "relation = EXCLUDED.relation, raw_type = EXCLUDED.raw_type, source = EXCLUDED.source, note = EXCLUDED.note, "
                "prompt_version = EXCLUDED.prompt_version, created_at = now()",
                (sid, law, d["text_md5"], d["relation"], d.get("raw_type"), d.get("reason"), PROMPT_VERSION))
            applied += 1
        conn.commit()
    print(f"applied {applied} referee decisions, skipped {skipped}")


if __name__ == "__main__":
    main()
