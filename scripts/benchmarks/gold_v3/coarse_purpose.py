#!/usr/bin/python3
"""Purpose as the law's anatomy: a decision list (PURPOSE-CLASSIFICATION.md § Purpose is the law's anatomy, 2026-10-07).

Thirteen members tested in precedence order, first match wins (Rivest's decision lists; a table of precedence with a
"not elsewhere classified" default, as in Dewey/UDC and ISIC/NACE). Purpose is the topic facet only: whether the
sentence creates a duty or a power is the DRRP facet and never decides purpose. A provision takes the first member
whose cue fires; a list item with no cue of its own takes its nearest stem's member; otherwise **Substantive
requirements**, the default. Empty text is `undetermined`. The Rust regex tier is the production home once the cues
settle.

  coarse_purpose.py              # score against Gemini v1.3 purposes (6,959) mapped to the members
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

DEFAULT = "Substantive requirements"
# the members in precedence order (first match wins); the default comes last
CLASSES = [
    "Citation and commencement", "Amendment and revocation", "Interpretation", "Subordinate legislation",
    "Application, exemption and transition", "Review", "Appeals, compensation and defences", "Offences and penalties",
    "Enforcement", "Bodies and their functions", "Fees and charges", "Financial provisions", DEFAULT,
]
# cue labels finer than the members (Transitional merged into Application, 2026-10-06); kept so the gold selection's
# targets stay reproducible
COARSE_OF = {"Application and exemption": "Application, exemption and transition",
             "Transitional and saving": "Application, exemption and transition"}


def coarse(label: str) -> str:
    return COARSE_OF.get(label, label)


# the 18 published values -> members (Requirement/Power Conferred have no member of their own: the default)
FROM_18 = {
    "Enactment+Citation+Commencement": "Citation and commencement", "Extent": "Citation and commencement",
    "Interpretation+Definition": "Interpretation",
    "Application+Scope": "Application, exemption and transition", "Exemption": "Application, exemption and transition",
    "Requirement": DEFAULT, "Procedure+Detail": DEFAULT, "Power Conferred": DEFAULT,
    "Enforcement+Prosecution": "Enforcement", "Offence": "Offences and penalties",
    "Liability": "Appeals, compensation and defences", "Defence+Appeal": "Appeals, compensation and defences",
    "Charge+Fee": "Fees and charges",
    "Amendment": "Amendment and revocation", "Repeal+Revocation": "Amendment and revocation",
    "Transitional Arrangement": "Application, exemption and transition",
    "Establishment+Constitution": "Bodies and their functions",
}

# the decision list: ordered as CLASSES (the default has no cue)
CUES = [
    ("Citation and commencement", r"\bmay be cited as\b|\bcomes? into (?:force|operation)\b|\bshall come into (?:force|operation)\b|\bshall enter into force\b"
                                  r"|\bextends? (?:only )?to (?:England|Wales|Scotland|Northern Ireland|Great Britain|the United Kingdom)"
                                  r"|\bshall bring into force the laws, regulations and administrative provisions\b"),
    # direct amending text only: a power to amend by regulations is Subordinate legislation
    ("Amendment and revocation", r"\b(?:is|are) hereby (?:revoked|repealed)\b|\b(?:Regulations|Order|Act|Rules|Directive|provisions?)\b[^.;]{0,60}\b(?:is|are) (?:revoked|repealed)\b|\bfor .{1,80} substitute\b|\bthere (?:is|are|shall be) inserted\b"
                                 r"|\b(?:is|are) amended as follows\b|\bomit\b"),
    ("Interpretation", r"^In (?:this|these) [^,]{0,40},|“[^”]{1,80}” (?:means|includes|has the (?:same )?meaning)"
                       r"|\breferences? (?:in [^,]{0,60})?to .{1,80} (?:is|are) to be (?:read|construed)\b|\bshall be construed as\b"),
    # powers to make further law (Interpretation Act 1978 s.21; Jason 2026-10-07)
    ("Subordinate legislation", r"\b(?:may|shall|must) by (?:regulations|order|rules|scheme|byelaws|statutory instrument)\b"
                                r"|\bpower to make (?:regulations|an order|orders|rules|byelaws)\b|\bmay make (?:regulations|rules|byelaws|an order)\b"
                                r"|\b(?:regulations|orders?|rules|byelaws|schemes?) (?:under|made under) (?:this|subsection|section|paragraph|article|regulation)\b[^.;]{0,80}\b(?:may|shall|must)\b"
                                r"|\b(?:delegated|implementing) acts?\b|\bstatutory instrument containing\b|\bexercisable by statutory instrument\b"
                                r"|\bsubject to annulment\b|\bresolution of (?:each|either) House\b|\bdraft of (?:the )?(?:regulations|order|instrument)\b"),
    ("Transitional and saving", r"\bcontinues? to have effect\b|\bas if (?:this|these|that) .{0,40}had not\b|\btransitional\b|\bsaving\b"),
    ("Application and exemption", r"\b(?:shall|do|does) not apply\b|\bapplies? (?:only )?(?:to|in relation to)\b|\bshall apply (?:to|in relation to)\b"
                                  r"|\bnothing in (?:this|these)\b.{0,60}\bappl|\bexempt|\bbinds? the Crown\b|\bis addressed to the Member States\b"),
    ("Review", r"\bmust from time to time\b[^.;]{0,40}\b(?:carry out a )?review\b|\bcarry out a review of the regulatory provision\b|\bpublish a report setting out the conclusions of the review\b"),
    ("Appeals, compensation and defences", r"\bit (?:is|shall be) a defence\b|\bmay appeal\b|\ban appeal (?:lies|shall lie)\b|\bcompensation\b"),
    ("Offences and penalties", r"\b(?:is|shall be) guilty of an offence\b|\bcommits an offence\b|\bliable,? on (?:summary )?conviction\b"
                               r"|\bliable to (?:a fine|imprisonment)\b|\bcivil sanction"),
    ("Enforcement", r"\binspectors?\b|\benforcing authorit|\bimprovement notice\b|\bprohibition notice\b|\benforcement notice\b"
                    r"|\bpower(?:s)? of entry\b|\benter (?:any )?premises\b|\btake samples\b|\bseize\b|\bfixed penalty\b"),
    ("Bodies and their functions", r"\bthere shall (?:continue to )?be a body\b|\bis (?:hereby )?established\b|\b(?:body|committee|board|panel|council|commission|tribunal|authority)\b[^.;]{0,40}\bshall consist of\b|\bshall be a body corporate\b"
                                   r"|\bthe functions of\b|\b(?:may|must|shall) (?:issue|give|publish) guidance\b|\bcode of practice\b"),
    ("Fees and charges", r"\bfees?\b.{0,40}\b(?:payable|shall be paid|charge)|\bmay charge\b|\bshall pay\b.{0,40}\bfee"),
    ("Financial provisions", r"\bmoney provided by Parliament\b|\bexpenses (?:incurred|of)\b[^.;]{0,60}\b(?:shall|are to) be (?:paid|defrayed)\b|\bConsolidated Fund\b"),
]
_CUES = [(name, re.compile(rx, re.I | re.M)) for name, rx in CUES]


def cue(text: str) -> str | None:
    """The first member whose cue fires (None: no member claims it)."""
    t = (text or "").strip()
    return next((name for name, rx in _CUES if rx.search(t)), None)


def classify(section_id: str, texts: dict, inherit: bool = True) -> tuple[str, str]:
    """(member, how): how = 'cue' | 'stem' | 'default' | 'none' (no text)."""
    own = cue(texts.get(section_id, ""))
    if own:
        return own, "cue"
    if inherit:
        for a in ancestors(section_id):
            if a in texts:
                c = cue(texts[a])
                if c:
                    return c, "stem"
    return (DEFAULT, "default") if (texts.get(section_id) or "").strip() else ("undetermined", "none")


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
        k = coarse(k)
        how_n[how] += 1
        n_gold[g] += 1
        pred[k] += 1
        hit[k] += k == g
        if k != g:
            wrong[k][g] += 1
    print(f"{len(gold):,} labelled provisions; decided by cue {how_n['cue']:,}, stem {how_n['stem']:,}, default {how_n['default']:,}, no text {how_n['none']:,}")
    print(f"{'class':28}{'labels':>7}{'pred':>6}{'prec':>6}{'recall':>7}  most common wrong")
    for k in CLASSES + ["undetermined"]:
        pr = hit[k] / pred[k] if pred[k] else 0
        rc = hit[k] / n_gold[k] if n_gold[k] else 0
        print(f"{k:28}{n_gold[k]:>7}{pred[k]:>6}{pr:>6.0%}{rc:>7.0%}  {wrong[k].most_common(2)}")


if __name__ == "__main__":
    main()
