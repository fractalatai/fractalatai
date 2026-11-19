#!/usr/bin/env python3
"""Impairment axis: compute life_altering blocks from injury-type mix x P(meets IOGP).

P(life-altering | band, region) = sum over injury types n of
    P(n | event, region) x P(meets IOGP | n)

Inputs (data/sif/calibration/impairment.json):
  p_meets_iogp  P(meets the IOGP permanent impairment criteria | injury type)
  assumptions   shares where BLS injury types are broader than the outcome studies
  bls.events    BLS SOII 2023-24 DART counts by event (nature and part marginals)
  band_map      mechanism/band -> BLS event key

Regions (life_altering.REGIONS):
  hand_finger  amputations (all placed here, upper bound) + fractures x P(hand)
               + tendon and nerve shares (Sorock 2002) + grafted burns
  eye          serious-eye share x P(eye) + grafted burns
  head         head fractures as moderate/severe brain injury + concussion share
               x P(mild) + grafted burns (head excludes the eye)
  trunk        fractures x P(spine) + grafted burns
  limb         fractures x P(limb) + grafted burns

Nature is assumed independent of region within an event (BLS gives marginals).
The band level is the region-weighted mean over cases with a single region.
Ranges: every input at its low, then its high, end.

Usage:
    /usr/bin/python3 scripts/sif/impairment.py            # print the table
    /usr/bin/python3 scripts/sif/impairment.py --write    # update calibration blocks
Decided: Jason, 2026-11-19. See .claude/sessions/sif/11-18-26-sif-model-improvements.md.
"""

import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CAL = ROOT / "data" / "sif" / "calibration"
TABLE = CAL / "impairment.json"
FILES = {"caught_in": "caught_in.json", "abrasion": "abrasion.json", "thermal": "thermal.json",
         "chemical": "chemical.json", "motion_struck": "motion_struck.json",
         "falls_gravity": "falls_gravity.json", "slip_no_fall": "slip_no_fall.json"}
ORDER = ["hand_finger", "eye", "head", "trunk", "limb"]


def _n(x):
    return x or 0


def regions_of(ev):
    p = ev["part"]
    hand, eye = _n(p.get("hand_finger")), _n(p.get("eye"))
    return {"hand_finger": hand, "eye": eye,
            "head": max(_n(p.get("head_incl_eye")) - eye, 0),
            "trunk": _n(p.get("trunk")),
            "limb": max(_n(p.get("upper_extremities")) - hand, 0) + _n(p.get("lower_extremities"))}


def region_values(ev, pm, a):
    """P(life-altering | region) for one event, with pm/a as plain {key: value} dicts."""
    T, nat, R = ev["dart_cases"], ev["nature"], regions_of(ev)
    fr = _n(nat.get("fractures")) / T
    burns = (_n(nat.get("thermal_burns")) + _n(nat.get("chemical_burns"))) / T
    grafted = burns * a["burn_graft_share"] * pm["burn_grafted"]
    out = {
        "limb": fr * pm["fracture_limb"] + grafted,
        "trunk": fr * pm["fracture_spine"] + grafted,
        "head": fr * pm["tbi_moderate_severe"] + a["head_mild_tbi_share"] * pm["tbi_mild"] + grafted,
        "eye": a["eye_serious_share"] * pm["eye_serious"] + grafted,
    }
    if R["hand_finger"]:
        out["hand_finger"] = (_n(nat.get("amputations")) / R["hand_finger"] * pm["amputation"]
                              + fr * pm["fracture_hand"]
                              + a["hand_tendon_share"] * (pm["tendon_flexor"] + pm["tendon_extensor"]) / 2
                              + a["hand_nerve_share"] * pm["nerve_digital"] + grafted)
    return {k: min(1.0, v) for k, v in out.items()}, R


def _params(table, which):
    """which: 'p', 0 (low) or 1 (high)."""
    pick = (lambda d: d["p"]) if which == "p" else (lambda d: d.get("range", [d["p"], d["p"]])[which])
    pm = {k: pick(v) for k, v in table["p_meets_iogp"].items()}
    a = {k: pick(v) for k, v in table["assumptions"].items() if "p" in v}
    return pm, a


def block_for(table, event_key):
    ev = table["bls"]["events"][event_key]
    mid, R = region_values(ev, *_params(table, "p"))
    lo, _ = region_values(ev, *_params(table, 0))
    hi, _ = region_values(ev, *_params(table, 1))
    tot = sum(R.values())
    split = {k: round(R[k] / tot, 2) for k in ORDER}
    band_level = sum(R[k] / tot * mid.get(k, 0.0) for k in ORDER)
    return {
        "status": table["status"],
        "decided": table["decided"],
        "method": "impairment.json: injury-type mix x P(meets IOGP)",
        "regions": {k: {"p": round(mid[k], 3), "range": [round(lo[k], 3), round(hi[k], 3)]}
                    for k in ORDER if k in mid},
        "region_split": split,
        "split_source": f"BLS SOII 2023-24 biennial DART counts, {ev['label']}",
        "band_level": round(band_level, 3),
    }


# --- Text splice: the calibration files are hand-formatted, so edit in place ---

def _match(text, i, open_ch, close_ch):
    """Index just past the bracket that closes the one at text[i]; string-aware."""
    depth, j, in_str = 0, i, False
    while j < len(text):
        c = text[j]
        if in_str:
            if c == "\\":
                j += 1
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == open_ch:
            depth += 1
        elif c == close_ch:
            depth -= 1
            if depth == 0:
                return j + 1
        j += 1
    raise ValueError("unbalanced")


def splice(text, band, block, indent=6):
    m = re.search(r'"band":\s*"%s"' % re.escape(band), text)
    if not m:
        raise KeyError(band)
    start = text.rfind("{", 0, m.start())
    end = _match(text, start, "{", "}")
    obj = text[start:end]
    body = json.dumps(block, indent=2, ensure_ascii=False).replace("\n", "\n" + " " * indent)
    k = obj.find('"life_altering"')
    if k >= 0:
        v = obj.index("{", k)
        obj = obj[:v] + body + obj[_match(obj, v, "{", "}"):]
    else:
        close = obj.rstrip().rfind("}")
        head = obj[:close].rstrip()
        obj = head + ",\n" + " " * indent + '"life_altering": ' + body + "\n" + " " * (indent - 2) + "}"
    out = text[:start] + obj + text[end:]
    json.loads(out)  # still valid JSON
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--write", action="store_true", help="Update life_altering blocks in the calibration files")
    args = ap.parse_args()
    table = json.loads(TABLE.read_text(encoding="utf-8"))
    texts = {}
    print(f"{'band':38} {'event':6} {'level':>6}  " + "  ".join(f"{r:>11}" for r in ORDER))
    for key, ev in table["band_map"].items():
        mech, band = key.split("/")
        blk = block_for(table, ev)
        if key in table.get("notes", {}):
            blk["note"] = table["notes"][key]
        print(f"{key:38} {ev:6} {blk['band_level']:6.3f}  "
              + "  ".join(f"{blk['regions'].get(r, {}).get('p', 0):11.3f}" for r in ORDER))
        if args.write:
            f = CAL / FILES[mech]
            texts.setdefault(f, f.read_text(encoding="utf-8"))
            texts[f] = splice(texts[f], band, blk)
    for f, t in texts.items():
        f.write_text(t, encoding="utf-8")
        print(f"wrote {f.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
