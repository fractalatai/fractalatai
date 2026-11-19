"""Life-altering axis for the SIF AI model (body-region rule).

P(serious) = max(P(threat to life), P(life-altering | band, region)).

Threat to life is the existing metalog P(SIF) from a band's p10/p50/p90.
Life-altering comes from the band's optional `life_altering` block in the
calibration JSON:

    "life_altering": {
      "status": "provisional",
      "regions": {"hand_finger": {"p": 0.06, "range": [0.03, 0.15]}},
      "region_split": {"hand_finger": 0.47},
      "band_level": 0.03,
      ...
    }

Region rule (Jason, 2026-11-18): region-conditional, with the band-level
value used when no region is stated. A stated region that the block does
not list contributes nothing on this axis (threat to life still applies).
Values: computed by impairment.py (injury-type mix x P(meets IOGP), from
data/sif/calibration/impairment.json), except the laser band. Jason, 2026-11-19. See
.claude/sessions/sif/11-18-26-sif-model-improvements.md.
"""

import re

REGIONS = ["hand_finger", "eye", "head", "trunk", "limb", "unknown"]

# First match wins: eye before head, hand/finger before limb.
_REGION_KEYWORDS = [
    ("eye", ("eye", "ocular", "retina", "cornea", "vision", "sight")),
    ("hand_finger", ("finger", "thumb", "hand", "knuckle", "palm", "digit")),
    ("head", ("head", "face", "skull", "scalp", "ear", "jaw", "nose")),
    ("trunk", ("trunk", "torso", "chest", "abdomen", "back", "spine", "hip", "pelvis")),
    ("limb", ("arm", "wrist", "elbow", "shoulder", "leg", "knee", "ankle", "foot", "feet", "toe", "limb")),
]


def norm_region(value):
    """Normalise a pass-2 `body_region_at_risk` value; None if not usable."""
    v = str(value or "").strip().lower().replace("/", "_").replace(" ", "_")
    v = {"hand": "hand_finger", "finger": "hand_finger", "fingers": "hand_finger",
         "hand_fingers": "hand_finger", "hands": "hand_finger", "eyes": "eye",
         "limbs": "limb", "torso": "trunk"}.get(v, v)
    return v if v in REGIONS else None


def region_from_text(text):
    """Fallback: keyword scan of free text (e.g. pass-1 body_vulnerability)."""
    t = str(text or "").lower()
    for region, kws in _REGION_KEYWORDS:
        if any(re.search(r"\b" + kw, t) for kw in kws):
            return region
    return "unknown"


def p_life_altering(band, region):
    """P(life-altering) for a calibration band dict, or None if the band has no block."""
    la = band.get("life_altering")
    if not la:
        return None
    if not region or region == "unknown":
        return la["band_level"]
    r = la["regions"].get(region)
    return r["p"] if r else 0.0


def p_serious(p_threat_to_life, band, region):
    """Return (p_serious, p_life_altering, driven_by)."""
    pla = p_life_altering(band, region)
    if pla is not None and pla > p_threat_to_life:
        return pla, pla, "life_altering"
    return p_threat_to_life, pla, "threat_to_life"
