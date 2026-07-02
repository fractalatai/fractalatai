#!/usr/bin/env /usr/bin/python3
"""Batch-generate golden benchmarks for all QQ families.

Runs generate_benchmarks.py for each family's selected Act + SI,
capping at 150 provisions per law to control cost.

Usage:
    source ~/.bashrc
    GEMINI_API_KEY="$GEMINI_API_KEY" /usr/bin/python3 scripts/generate_benchmarks_batch.py
"""

import os
import subprocess
import sys

BATCH = [
    ("CLIMATE CHANGE", "UK_uksi_2015_310"),
    ("ENERGY", "UK_ukpga_2004_20"),
    ("ENERGY", "UK_uksi_2014_1643"),
    ("ENVIRONMENTAL PROTECTION", "UK_ukpga_1990_43"),
    ("ENVIRONMENTAL PROTECTION", "UK_uksi_2016_1154"),
    ("FIRE", "UK_uksi_2005_1541"),
    ("FIRE: Dangerous and Explosive Substances", "UK_uksi_2014_1638"),
    ("HEALTH: Public", "UK_asp_2005_13"),
    ("HR: Employment", "UK_ukpga_1996_18"),
    ("HR: Employment", "UK_uksi_2002_2788"),
    ("NUCLEAR & RADIOLOGICAL", "UK_ukpga_1993_12"),
    ("NUCLEAR & RADIOLOGICAL", "UK_eudr_2013_59"),
    ("OH&S: Gas & Electrical Safety", "UK_uksi_2016_1101"),
    ("PLANNING & INFRASTRUCTURE", "UK_ukpga_1990_10"),
    ("PLANNING & INFRASTRUCTURE", "UK_uksi_2017_572"),
    ("POLLUTION", "UK_ukpga_1974_40"),
    ("POLLUTION", "UK_uksi_2006_1380"),
    ("PUBLIC: Building Safety", "UK_uksi_2010_2214"),
    ("PUBLIC: Consumer / Product Safety", "UK_eudr_2014_68"),
    ("TOWN & COUNTRY PLANNING", "UK_ukpga_1997_8"),
    ("WASTE", "UK_uksi_2009_890"),
    ("WATER & WASTEWATER", "UK_ukpga_1991_56"),
    ("WATER & WASTEWATER", "UK_uksi_1999_1148"),
    ("WILDLIFE & COUNTRYSIDE", "UK_ukpga_1981_69"),
    ("WILDLIFE & COUNTRYSIDE", "UK_uksi_2017_1012"),
]

MAX_PROVISIONS = 150

def main():
    api_key = os.environ.get("GEMINI_API_KEY")
    if not api_key:
        print("Error: GEMINI_API_KEY not set")
        sys.exit(1)

    total = len(BATCH)
    success = 0
    failed = []

    for i, (family, law) in enumerate(BATCH):
        print(f"\n[{i+1}/{total}] {family} — {law}")
        try:
            result = subprocess.run(
                [
                    "/usr/bin/python3", "scripts/generate_benchmarks.py",
                    "--law", law,
                    "--family", family,
                ],
                capture_output=True, text=True, timeout=600,
                env={**os.environ, "GEMINI_API_KEY": api_key},
            )
            # Print last 3 lines of output
            lines = result.stdout.strip().split('\n')
            for line in lines[-3:]:
                print(f"  {line}")
            if result.returncode == 0:
                success += 1
            else:
                print(f"  ERROR: {result.stderr[-200:]}")
                failed.append((family, law))
        except subprocess.TimeoutExpired:
            print(f"  TIMEOUT")
            failed.append((family, law))
        except Exception as e:
            print(f"  ERROR: {e}")
            failed.append((family, law))

    print(f"\n{'='*60}")
    print(f"Batch complete: {success}/{total} succeeded")
    if failed:
        print(f"Failed:")
        for fam, law in failed:
            print(f"  {fam} — {law}")


if __name__ == "__main__":
    main()
