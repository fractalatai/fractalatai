#!/usr/bin/env python3
"""Build SIF taxonomy from ICD-11 Simple Tabulation data.

Extracts Chapter 23 (External causes) and Extension Codes (Dimensions of
external causes) into structured JSON for the SIF classifier and simulator.

Also builds the ICECI C2 → Energy Wheel mapping table.

Usage:
    /usr/bin/python3 scripts/sif/build_taxonomy.py
"""

import csv
import json
from pathlib import Path

DATA_DIR = Path("data/sif/taxonomy")
CH23_FILE = DATA_DIR / "icd11-chapter23-external-causes.tsv"
EXT_FILE = DATA_DIR / "icd11-external-causes-extensions.tsv"
OUTPUT_DIR = DATA_DIR

# Column indices in the Simple Tabulation TSV
COL_FOUNDATION_URI = 0
COL_LINEARIZATION_URI = 1
COL_CODE = 2
COL_BLOCK_ID = 3
COL_TITLE = 4
COL_CLASS_KIND = 5
COL_DEPTH = 6
COL_IS_RESIDUAL = 7
COL_CHAPTER = 8


def clean_title(title: str) -> str:
    """Remove leading dashes and quotes from title."""
    t = title.strip().strip('"').strip()
    while t.startswith("- "):
        t = t[2:]
    return t.strip()


def parse_tsv(path: Path) -> list[dict]:
    """Parse ICD-11 Simple Tabulation TSV into list of dicts."""
    rows = []
    with open(path, encoding="utf-8-sig") as f:
        reader = csv.reader(f, delimiter="\t")
        for line in reader:
            if len(line) < 9:
                continue
            rows.append({
                "foundation_uri": line[COL_FOUNDATION_URI],
                "code": line[COL_CODE].strip(),
                "block_id": line[COL_BLOCK_ID].strip(),
                "title": clean_title(line[COL_TITLE]),
                "class_kind": line[COL_CLASS_KIND].strip(),
                "depth": int(line[COL_DEPTH]) if line[COL_DEPTH].strip().isdigit() else 0,
                "is_residual": line[COL_IS_RESIDUAL].strip() == "True",
            })
    return rows


def build_chapter23_taxonomy(rows: list[dict]) -> dict:
    """Build hierarchical taxonomy from Chapter 23 rows."""
    # Group into intent blocks (L1) → mechanism blocks (L2) → categories
    taxonomy = {"chapter": "23", "title": "External causes of morbidity or mortality", "blocks": []}

    current_l1 = None
    current_l2 = None

    for r in rows:
        if r["class_kind"] == "chapter":
            continue

        if r["class_kind"] == "block" and r["depth"] == 1:
            current_l1 = {
                "block_id": r["block_id"],
                "title": r["title"],
                "sub_blocks": [],
            }
            taxonomy["blocks"].append(current_l1)
            current_l2 = None

        elif r["class_kind"] == "block" and r["depth"] == 2 and current_l1:
            current_l2 = {
                "block_id": r["block_id"],
                "title": r["title"],
                "codes": [],
            }
            current_l1["sub_blocks"].append(current_l2)

        elif r["class_kind"] == "category" and r["code"] and current_l2:
            current_l2["codes"].append({
                "code": r["code"],
                "title": r["title"],
                "is_residual": r["is_residual"],
            })

    return taxonomy


def build_extension_taxonomy(rows: list[dict]) -> dict:
    """Build taxonomy from Extension Codes (Dimensions of external causes)."""
    taxonomy = {"title": "Dimensions of external causes", "dimensions": []}

    current_dim = None
    current_sub = None

    for r in rows:
        if r["class_kind"] == "block" and r["depth"] == 1:
            # This is the top-level "Dimensions of external causes" block itself
            continue

        if r["class_kind"] == "block" and r["depth"] == 2:
            current_dim = {
                "block_id": r["block_id"],
                "title": r["title"],
                "sub_blocks": [],
                "codes": [],
            }
            taxonomy["dimensions"].append(current_dim)
            current_sub = None

        elif r["class_kind"] == "block" and r["depth"] == 3 and current_dim:
            current_sub = {
                "block_id": r["block_id"],
                "title": r["title"],
                "codes": [],
            }
            current_dim["sub_blocks"].append(current_sub)

        elif r["class_kind"] == "category" and r["code"]:
            target = current_sub if current_sub else current_dim
            if target:
                target["codes"].append({
                    "code": r["code"],
                    "title": r["title"],
                    "is_residual": r["is_residual"],
                })

    return taxonomy


def build_mechanism_energy_mapping() -> list[dict]:
    """Build the ICECI C2 → Energy Wheel mapping table.

    Maps ICD-11 Chapter 23 unintentional mechanism blocks to Energy Wheel
    categories and SIF gate assignments.
    """
    return [
        {
            "icd11_block": "BlockL2-PA0",
            "iceci_c2": "1.1",
            "mechanism": "Transport injury event",
            "energy_types": ["motion"],
            "sif_gate": "NEEDS_ASSESSMENT",
        },
        {
            "icd11_block": "BlockL2-PA6",
            "iceci_c2": "1.5",
            "mechanism": "Fall",
            "energy_types": ["gravity"],
            "sif_gate": "NEEDS_ASSESSMENT",
        },
        {
            "icd11_block": "BlockL2-PA7",
            "iceci_c2": "1.2/1.3",
            "mechanism": "Contact with person, animal or plant",
            "energy_types": ["motion", "biological"],
            "sif_gate": "NEEDS_ASSESSMENT",
        },
        {
            "icd11_block": "BlockL2-PA8",
            "iceci_c2": "1.2/1.4/3.2",
            "mechanism": "Exposure to object (struck-by, caught-in, machinery)",
            "energy_types": ["motion", "mechanical"],
            "sif_gate": "NEEDS_ASSESSMENT",
        },
        {
            "icd11_block": "BlockL2-PA9",
            "iceci_c2": "5.2",
            "mechanism": "Immersion, submersion or falling into water",
            "energy_types": ["gravity", "pressure"],
            "sif_gate": "NEEDS_ASSESSMENT",
        },
        {
            "icd11_block": "BlockL2-PB0",
            "iceci_c2": "5.1/5.3",
            "mechanism": "Threat to breathing",
            "energy_types": ["pressure", "chemical"],
            "sif_gate": "NEEDS_ASSESSMENT",
        },
        {
            "icd11_block": "BlockL2-PB1",
            "iceci_c2": "4.1/4.2",
            "mechanism": "Exposure to thermal mechanism",
            "energy_types": ["thermal"],
            "sif_gate": "NEEDS_ASSESSMENT",
        },
        {
            "icd11_block": "BlockL2-PB2",
            "iceci_c2": "6.1/6.2",
            "mechanism": "Exposure to substances",
            "energy_types": ["chemical"],
            "sif_gate": "NEEDS_ASSESSMENT",
        },
        {
            "icd11_block": "BlockL2-PB5",
            "iceci_c2": "3.1/7.1/98.2",
            "mechanism": "Other mechanism (explosion, electricity, radiation, over-exertion)",
            "energy_types": ["pressure", "electrical", "radiation", "mechanical"],
            "sif_gate": "NEEDS_ASSESSMENT",
            "notes": "Over-exertion (7.1) sub-codes should be AUTO_NON_SIF; explosion/electrical are high-energy",
        },
        # Auto-non-SIF mechanisms (extracted from PB5 sub-codes)
        {
            "icd11_block": "PB5B",
            "iceci_c2": "7.1",
            "mechanism": "Over-exertion",
            "energy_types": [],
            "sif_gate": "AUTO_NON_SIF",
        },
        {
            "icd11_block": "PB59",
            "iceci_c2": "1.6",
            "mechanism": "Abrasion/friction",
            "energy_types": [],
            "sif_gate": "AUTO_NON_SIF",
        },
    ]


def main():
    print("Building SIF taxonomy from ICD-11 data...\n")

    # Parse Chapter 23
    ch23_rows = parse_tsv(CH23_FILE)
    ch23_taxonomy = build_chapter23_taxonomy(ch23_rows)
    ch23_path = OUTPUT_DIR / "icd11-chapter23-taxonomy.json"
    with open(ch23_path, "w") as f:
        json.dump(ch23_taxonomy, f, indent=2)
    n_blocks = sum(len(b["sub_blocks"]) for b in ch23_taxonomy["blocks"])
    n_codes = sum(
        len(c["codes"])
        for b in ch23_taxonomy["blocks"]
        for c in b["sub_blocks"]
    )
    print(f"Chapter 23: {len(ch23_taxonomy['blocks'])} intent blocks, {n_blocks} mechanism blocks, {n_codes} leaf codes")
    print(f"  → {ch23_path}")

    # Parse Extension Codes
    ext_rows = parse_tsv(EXT_FILE)
    ext_taxonomy = build_extension_taxonomy(ext_rows)
    ext_path = OUTPUT_DIR / "icd11-extension-codes-taxonomy.json"
    with open(ext_path, "w") as f:
        json.dump(ext_taxonomy, f, indent=2)
    n_dims = len(ext_taxonomy["dimensions"])
    n_ext_codes = sum(
        len(d.get("codes", []))
        + sum(len(s.get("codes", [])) for s in d.get("sub_blocks", []))
        for d in ext_taxonomy["dimensions"]
    )
    print(f"Extension Codes: {n_dims} dimensions, {n_ext_codes} codes")
    print(f"  → {ext_path}")

    # Build ICECI → Energy Wheel mapping
    mapping = build_mechanism_energy_mapping()
    mapping_path = OUTPUT_DIR / "mechanism-energy-mapping.json"
    with open(mapping_path, "w") as f:
        json.dump(mapping, f, indent=2)
    print(f"\nMechanism → Energy mapping: {len(mapping)} entries")
    print(f"  → {mapping_path}")

    # Summary of unintentional mechanism blocks for the classifier label set
    unintentional = ch23_taxonomy["blocks"][0]  # First L1 block
    print(f"\nUnintentional causes — classifier label candidates:")
    for sub in unintentional["sub_blocks"]:
        n = len(sub["codes"])
        print(f"  {sub['block_id']}: {sub['title']} ({n} codes)")


if __name__ == "__main__":
    main()
