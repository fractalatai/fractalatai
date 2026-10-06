#!/usr/bin/python3
"""Coarse purpose layer, first cut (PURPOSE-CLASSIFICATION.md § Layered purpose, decided 2026-10-06).

Twelve classes in statutory terms. A provision takes the first class whose text cue fires. A list item
with no cue of its own inherits its nearest stem's class. Otherwise it is `undetermined` (escalates to
the fine layer). This is the measurement prototype for phase 2. The Rust regex tier is the production
home once the cues settle.

  coarse_purpose.py              # score against Gemini v1.3 purposes (6,959) mapped to coarse classes
  coarse_purpose.py --no-inherit # cues only
"""

import argparse
import collections
import os
import re
import sys

ROOT = "/var/home/jason/fractalaw"
sys.path.insert(0, os.path.join(ROOT, "scripts"))
from drrp_prompt import ancestors  # noqa: E402

PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"

CLASSES = [
    "Citation and commencement", "Interpretation", "Application and exemption", "Duties", "Powers",
    "Enforcement", "Offences and penalties", "Appeals and defences", "Fees and charges",
    "Amendment and revocation", "Transitional and saving", "Constitution",
]

# the 18 published values -> coarse class
FROM_18 = {
    "Enactment+Citation+Commencement": "Citation and commencement", "Extent": "Citation and commencement",
    "Interpretation+Definition": "Interpretation",
    "Application+Scope": "Application and exemption", "Exemption": "Application and exemption",
    "Requirement": "Duties", "Procedure+Detail": "Duties", "Power Conferred": "Powers",
    "Enforcement+Prosecution": "Enforcement", "Offence": "Offences and penalties", "Liability": "Offences and penalties",
    "Defence+Appeal": "Appeals and defences", "Charge+Fee": "Fees and charges",
    "Amendment": "Amendment and revocation", "Repeal+Revocation": "Amendment and revocation",
    "Transitional Arrangement": "Transitional and saving", "Establishment+Constitution": "Constitution",
}

# ordered: first match wins, so specific classes come before Duties/Powers
CUES = [
    ("Citation and commencement", r"\bmay be cited as\b|\bcomes? into (?:force|operation)\b|\bshall come into (?:force|operation)\b"
                                  r"|\bextends? (?:only )?to (?:England|Wales|Scotland|Northern Ireland|Great Britain|the United Kingdom)"),
    ("Amendment and revocation", r"\b(?:is|are) (?:hereby )?(?:revoked|repealed)\b|\bfor .{1,80} substitute\b|\bthere (?:is|are|shall be) inserted\b"
                                 r"|\b(?:is|are) amended as follows\b|\bomit\b"),
    ("Transitional and saving", r"\bcontinues? to have effect\b|\bas if (?:this|these|that) .{0,40}had not\b|\btransitional\b|\bsaving\b"),
    ("Interpretation", r"^In (?:this|these) [^,]{0,40},|“[^”]{1,80}” (?:means|includes|has the (?:same )?meaning)"
                       r"|\breferences? (?:in [^,]{0,60})?to .{1,80} (?:is|are) to be (?:read|construed)\b|\bshall be construed as\b"),
    ("Offences and penalties", r"\b(?:is|shall be) guilty of an offence\b|\bcommits an offence\b|\bliable,? on (?:summary )?conviction\b"
                               r"|\bliable to (?:a fine|imprisonment)\b"),
    ("Appeals and defences", r"\bit (?:is|shall be) a defence\b|\bmay appeal\b|\ban appeal (?:lies|shall lie)\b"),
    ("Constitution", r"\bthere shall (?:continue to )?be a body\b|\bis (?:hereby )?established\b|\bshall consist of\b|\bshall be a body corporate\b"),
    ("Enforcement", r"\binspectors?\b|\benforcing authorit|\bimprovement notice\b|\bprohibition notice\b|\benforcement notice\b"
                    r"|\bpower(?:s)? of entry\b|\benter (?:any )?premises\b|\btake samples\b|\bseize\b"),
    ("Application and exemption", r"\b(?:shall|do|does) not apply\b|\bapplies? (?:only )?(?:to|in relation to)\b|\bshall apply (?:to|in relation to)\b"
                                  r"|\bnothing in (?:this|these)\b.{0,60}\bappl|\bexempt"),
    ("Fees and charges", r"\bfees?\b.{0,40}\b(?:payable|shall be paid|charge)|\bmay charge\b|\bshall pay\b.{0,40}\bfee"),
    ("Duties", r"\b(?:shall|must)\b|\bit shall be the duty\b|\bis required to\b|\bfunctions\b"),
    ("Powers", r"\bmay\b"),
]
_CUES = [(name, re.compile(rx, re.I | re.M)) for name, rx in CUES]


def cue(text: str) -> str | None:
    t = (text or "").strip()
    return next((name for name, rx in _CUES if rx.search(t)), None)


def classify(section_id: str, texts: dict, inherit: bool = True) -> tuple[str, str]:
    """(class, how): how = 'cue' | 'stem' | 'none'."""
    own = cue(texts.get(section_id, ""))
    if own:
        return own, "cue"
    if inherit:
        for a in ancestors(section_id):
            if a in texts:
                c = cue(texts[a])
                if c:
                    return c, "stem"
    return "undetermined", "none"


def main() -> None:
    import psycopg2
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--no-inherit", action="store_true")
    args = ap.parse_args()
    cur = psycopg2.connect(PG).cursor()
    cur.execute("""SELECT DISTINCT ON (section_id) section_id, response->>'purpose' FROM drrp_training_labels_raw
                   WHERE model = 'gemini-3.8-flash' AND error IS NULL AND prompt_version LIKE 'drrp-v1.3%%'
                   ORDER BY section_id, created_at DESC""")
    gold = {sid: FROM_18.get(p, "?") for sid, p in cur.fetchall()}
    laws = sorted({s.split(":")[0] for s in gold})
    cur.execute("SELECT section_id, text FROM legislation_text WHERE law_name = ANY(%s)", (laws,))
    texts = dict(cur.fetchall())
    pred, hit, n_gold, how_n = (collections.Counter() for _ in range(4))
    wrong = collections.defaultdict(collections.Counter)
    for sid, g in gold.items():
        k, how = classify(sid, texts, not args.no_inherit)
        how_n[how] += 1
        n_gold[g] += 1
        pred[k] += 1
        hit[k] += k == g
        if k != g:
            wrong[k][g] += 1
    print(f"{len(gold):,} labelled provisions; decided by cue {how_n['cue']:,}, stem {how_n['stem']:,}, undetermined {how_n['none']:,}")
    print(f"{'class':28}{'labels':>7}{'pred':>6}{'prec':>6}{'recall':>7}  most common wrong")
    for k in CLASSES + ["undetermined"]:
        pr = hit[k] / pred[k] if pred[k] else 0
        rc = hit[k] / n_gold[k] if n_gold[k] else 0
        print(f"{k:28}{n_gold[k]:>7}{pred[k]:>6}{pr:>6.0%}{rc:>7.0%}  {wrong[k].most_common(2)}")


if __name__ == "__main__":
    main()
