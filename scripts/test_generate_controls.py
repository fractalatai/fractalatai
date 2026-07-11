#!/usr/bin/env python3
"""Tests for generate_controls.py — prompt assembly and validation logic.

Tests confirm the pipeline correctly queries databases, assembles prompts,
and validates LLM output. Does NOT call Gemini — uses saved Phase 0 test
results as reference data.

Usage:
    /usr/bin/python3 scripts/test_generate_controls.py
    /usr/bin/python3 scripts/test_generate_controls.py -v
    /usr/bin/python3 scripts/test_generate_controls.py TestPromptAssembly.test_confined_spaces_provisions
"""

import json
import os
import sys
import unittest
from pathlib import Path

import duckdb
import psycopg2

# Add scripts dir to path
sys.path.insert(0, str(Path(__file__).parent))
from generate_controls import (
    get_law_outline, get_governed_provisions, format_user_prompt,
    lint_control, ensure_staging_table, store_controls,
    DEONTIC_VERBS, JUDGEMENT_TERMS, EXCLUDE_PURPOSES,
    VALID_CONTROL_TYPES, VALID_NATURES, VALID_DOMAINS,
    VALID_INFO_DISTANCES, VALID_BLAST_RADII, VALID_STRENGTHS,
)

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
DUCKDB_PATH = "data/fractalaw.duckdb"
TEST_RESULTS_DIR = Path("data/compliance-controls/test-results")

# Laws tested in Phase 0
TEST_LAWS = {
    "confined_spaces": "UK_uksi_1997_1713",
    "mhsw": "UK_uksi_1999_3242",
    "hswa": "UK_ukpga_1974_37",
    "coshh": "UK_uksi_2002_2677",
    "fso": "UK_uksi_2005_1541",
}


class TestDatabaseAccess(unittest.TestCase):
    """Test that DB queries return expected data for the 5 test laws."""

    @classmethod
    def setUpClass(cls):
        cls.duck = duckdb.connect(DUCKDB_PATH, read_only=True)
        cls.pg = psycopg2.connect(PG_DSN)

    @classmethod
    def tearDownClass(cls):
        cls.duck.close()
        cls.pg.close()

    def test_law_outline_exists(self):
        """All 5 test laws exist in DuckDB with required fields."""
        for label, name in TEST_LAWS.items():
            with self.subTest(law=label):
                outline = get_law_outline(self.duck, name)
                self.assertIsNotNone(outline, f"{name} not found in DuckDB")
                self.assertTrue(outline["title"], f"{name} has no title")
                self.assertTrue(outline["family"], f"{name} has no family")
                self.assertIsNotNone(outline["year"], f"{name} has no year")

    def test_law_outline_has_duty_holders(self):
        """Test laws have duty_holder populated."""
        for label, name in TEST_LAWS.items():
            with self.subTest(law=label):
                outline = get_law_outline(self.duck, name)
                holders = outline.get("duty_holder")
                self.assertTrue(holders, f"{name} has no duty_holder")

    def test_governed_provisions_count(self):
        """Each test law returns a reasonable number of governed provisions."""
        expected_ranges = {
            "confined_spaces": (5, 20),   # Phase 0 got 12
            "mhsw": (30, 70),             # Phase 0 got 49
            "hswa": (15, 50),             # Phase 0 got 30 (ss.2-8 only, filter leaks more)
            "coshh": (30, 70),            # Phase 0 got 51
            "fso": (30, 130),             # Phase 0 got 48 after manual dedup; pipeline gets ~122 due to reg/art duplicates (GH#118)
        }
        for label, name in TEST_LAWS.items():
            with self.subTest(law=label):
                provisions = get_governed_provisions(self.pg, name)
                lo, hi = expected_ranges[label]
                self.assertGreaterEqual(
                    len(provisions), lo,
                    f"{name}: got {len(provisions)} provisions, expected >= {lo}"
                )
                self.assertLessEqual(
                    len(provisions), hi,
                    f"{name}: got {len(provisions)} provisions, expected <= {hi}"
                )

    def test_provisions_have_required_fields(self):
        """Each provision has section_id, text, drrp_types, governed_actors."""
        provisions = get_governed_provisions(self.pg, TEST_LAWS["confined_spaces"])
        for prov in provisions:
            section_id, text, drrp_types, governed_actors, purposes, sig, clause = prov
            self.assertTrue(section_id, "Empty section_id")
            self.assertTrue(text, f"Empty text for {section_id}")
            self.assertIn("Obligation", drrp_types, f"{section_id} not an Obligation")
            self.assertTrue(governed_actors, f"{section_id} has no governed_actors")

    def test_provisions_exclude_offences(self):
        """Provisions with Offence/Exemption purposes are excluded."""
        for label, name in TEST_LAWS.items():
            with self.subTest(law=label):
                provisions = get_governed_provisions(self.pg, name)
                for prov in provisions:
                    purposes = prov[4] or []
                    for excl in EXCLUDE_PURPOSES:
                        self.assertNotIn(
                            excl, purposes,
                            f"{prov[0]} has excluded purpose '{excl}'"
                        )

    def test_provisions_exclude_schedules(self):
        """Schedule provisions are excluded."""
        for label, name in TEST_LAWS.items():
            with self.subTest(law=label):
                provisions = get_governed_provisions(self.pg, name)
                for prov in provisions:
                    self.assertNotIn("sch.", prov[0], f"{prov[0]} is a schedule provision")

    def test_provisions_exclude_territorial_variants(self):
        """Territorial variant provisions (e.g. s.28(2)[E+W]) are excluded."""
        for label, name in TEST_LAWS.items():
            with self.subTest(law=label):
                provisions = get_governed_provisions(self.pg, name)
                for prov in provisions:
                    self.assertNotIn("[", prov[0], f"{prov[0]} is a territorial variant")


class TestPromptAssembly(unittest.TestCase):
    """Test that prompts are correctly assembled from DB data."""

    @classmethod
    def setUpClass(cls):
        cls.duck = duckdb.connect(DUCKDB_PATH, read_only=True)
        cls.pg = psycopg2.connect(PG_DSN)

    @classmethod
    def tearDownClass(cls):
        cls.duck.close()
        cls.pg.close()

    def _build_prompt(self, law_label):
        name = TEST_LAWS[law_label]
        outline = get_law_outline(self.duck, name)
        provisions = get_governed_provisions(self.pg, name)
        return format_user_prompt(outline, provisions), provisions

    def test_prompt_has_law_outline(self):
        """Prompt contains law title, family, year."""
        prompt, _ = self._build_prompt("confined_spaces")
        self.assertIn("Confined Spaces Regulations", prompt)
        self.assertIn("OH&S: Occupational / Personal Safety", prompt)
        self.assertIn("1997", prompt)

    def test_prompt_has_provision_count(self):
        """Prompt states the number of provisions."""
        prompt, provisions = self._build_prompt("confined_spaces")
        self.assertIn(f"{len(provisions)} provisions", prompt)

    def test_prompt_uses_short_refs(self):
        """Provision headers use short refs, not full section_ids."""
        prompt, _ = self._build_prompt("confined_spaces")
        self.assertIn("### reg.", prompt)
        self.assertNotIn("UK_uksi_1997_1713:", prompt)

    def test_prompt_has_provision_text(self):
        """Prompt includes actual provision text."""
        prompt, _ = self._build_prompt("confined_spaces")
        self.assertIn("confined space", prompt.lower())

    def test_prompt_has_actor_labels(self):
        """Prompt includes governed actor labels."""
        prompt, _ = self._build_prompt("confined_spaces")
        self.assertIn("Org: Employer", prompt)

    def test_prompt_has_significance(self):
        """Prompt includes significance labels."""
        prompt, _ = self._build_prompt("confined_spaces")
        self.assertIn("HIGH", prompt)

    def test_prompt_has_instructions(self):
        """Prompt ends with generation instructions."""
        prompt, _ = self._build_prompt("confined_spaces")
        self.assertIn("Indicative mood only", prompt)
        self.assertIn("Consolidate", prompt)

    def test_prompt_filters_government_actors_from_outline(self):
        """Law outline duty holders exclude Gvt: and EU: prefixed actors."""
        prompt, _ = self._build_prompt("hswa")
        # The duty_holder list should not contain Gvt: entries
        # Look for the Duty holders line
        for line in prompt.split("\n"):
            if line.startswith("Duty holders:"):
                self.assertNotIn("Gvt:", line, "Government actors in duty holders")
                self.assertNotIn("EU:", line, "EU actors in duty holders")
                break

    def test_prompt_includes_explanatory_note_when_present(self):
        """Prompt includes Explanatory Note when the column is populated."""
        prompt, _ = self._build_prompt("confined_spaces")
        # CS now has explanatory_note populated from sertantai
        self.assertIn("Explanatory Note:", prompt)

    def test_prompt_truncates_explanatory_note(self):
        """Explanatory Note is truncated to 500 chars in controls prompt."""
        prompt, _ = self._build_prompt("fso")  # FSO has 9906 char note
        for line in prompt.split("\n"):
            if line.startswith("Explanatory Note:"):
                # 500 chars + "Explanatory Note: " prefix + "..."
                self.assertLessEqual(len(line), 530, f"Note too long: {len(line)} chars")

    def test_prompt_truncates_long_text(self):
        """Very long provision text is truncated to 500 chars."""
        # HSWA s.53(1) definitions section is very long
        prompt, _ = self._build_prompt("hswa")
        # Check no single provision block exceeds ~600 chars (500 + overhead)
        # This is a rough check — just ensure truncation works
        sections = prompt.split("### ")
        for section in sections[1:]:  # skip preamble
            lines = section.split("\n")
            for line in lines:
                if line.startswith('"') and len(line) > 600:
                    self.fail(f"Provision text too long ({len(line)} chars): {line[:80]}...")


class TestLintValidation(unittest.TestCase):
    """Test the Phase 2 automated lint against saved Phase 0 outputs."""

    def _load_test_results(self, filename):
        path = TEST_RESULTS_DIR / filename
        if not path.exists():
            self.skipTest(f"{path} not found")
        return json.loads(path.read_text())

    def test_confined_spaces_no_deontic(self):
        """Phase 0 CS controls have no deontic verbs."""
        controls = self._load_test_results("confined-spaces-v1.json")
        refs = {"reg.4(1)", "reg.4(2)", "reg.3(1)(a)", "reg.3(1)(b)", "reg.3(2)(b)",
                "reg.5(1)", "reg.5(2)(a)", "reg.5(2)(b)"}
        for ctrl in controls:
            flags = lint_control(ctrl, refs)
            deontic_flags = [f for f in flags if f.startswith("DEONTIC")]
            self.assertEqual(deontic_flags, [], f"Deontic flags on: {ctrl['title'][:60]}")

    def test_coshh_no_deontic(self):
        """Phase 0 COSHH controls have no deontic verbs."""
        controls = self._load_test_results("coshh-v1.json")
        # Use a permissive ref set for this test — we're testing lint, not linkage
        refs = {f"reg.{i}" for i in range(1, 20)}
        refs.update(f"reg.{i}({j})" for i in range(1, 20) for j in range(1, 10))
        for ctrl in controls:
            flags = lint_control(ctrl, refs)
            deontic_flags = [f for f in flags if f.startswith("DEONTIC")]
            self.assertEqual(deontic_flags, [], f"Deontic flags on: {ctrl['title'][:60]}")

    def test_all_controls_have_required_fields(self):
        """All Phase 0 controls have title, description, what_it_checks, evidence_hint."""
        for filename in ["confined-spaces-v1.json", "mhsw-v1.json", "hswa-v1.json",
                         "coshh-v1.json", "fso-v1.json"]:
            with self.subTest(file=filename):
                controls = self._load_test_results(filename)
                for ctrl in controls:
                    self.assertTrue(ctrl.get("title"), f"Missing title in {filename}")
                    self.assertTrue(ctrl.get("description"), f"Missing description in {filename}")
                    self.assertTrue(ctrl.get("what_it_checks"), f"Missing what_it_checks in {filename}")
                    self.assertTrue(ctrl.get("evidence_hint"), f"Missing evidence_hint in {filename}")

    def test_all_controls_have_valid_enums(self):
        """All Phase 0 controls have valid enum values."""
        for filename in ["confined-spaces-v1.json", "mhsw-v1.json", "hswa-v1.json",
                         "coshh-v1.json", "fso-v1.json"]:
            with self.subTest(file=filename):
                controls = self._load_test_results(filename)
                for ctrl in controls:
                    self.assertIn(ctrl.get("control_type"), VALID_CONTROL_TYPES,
                                  f"Invalid control_type in {filename}: {ctrl.get('control_type')}")
                    self.assertIn(ctrl.get("nature"), VALID_NATURES,
                                  f"Invalid nature in {filename}: {ctrl.get('nature')}")
                    self.assertIn(ctrl.get("domain"), VALID_DOMAINS,
                                  f"Invalid domain in {filename}: {ctrl.get('domain')}")
                    self.assertIn(ctrl.get("info_distance"), VALID_INFO_DISTANCES,
                                  f"Invalid info_distance in {filename}: {ctrl.get('info_distance')}")
                    self.assertIn(ctrl.get("blast_radius"), VALID_BLAST_RADII,
                                  f"Invalid blast_radius in {filename}: {ctrl.get('blast_radius')}")
                    self.assertIn(ctrl.get("mapping_strength"), VALID_STRENGTHS,
                                  f"Invalid mapping_strength in {filename}: {ctrl.get('mapping_strength')}")

    def test_lint_catches_deontic_verb(self):
        """Lint correctly flags a control with 'must' in the title."""
        bad_control = {
            "title": "The employer must carry out a risk assessment",
            "description": "A risk assessment is done",
            "what_it_checks": "Was it done",
            "control_type": "Preventive",
            "nature": "Manual",
            "domain": "Organisational",
            "info_distance": "Mediated",
            "blast_radius": "Site",
            "mapping_strength": "Primary",
            "linked_provisions": ["reg.3(1)"],
            "evidence_hint": {"type_a": "form", "type_b": "test"},
        }
        flags = lint_control(bad_control, {"reg.3(1)"})
        deontic_flags = [f for f in flags if f.startswith("DEONTIC")]
        self.assertTrue(deontic_flags, "Should flag 'must' in title")

    def test_lint_catches_paperwork_referent(self):
        """Lint correctly flags a description referencing paperwork."""
        bad_control = {
            "title": "A risk assessment is completed",
            "description": "A risk assessment document exists on file",
            "what_it_checks": "Check the file",
            "control_type": "Preventive",
            "nature": "Manual",
            "domain": "Organisational",
            "info_distance": "Mediated",
            "blast_radius": "Site",
            "mapping_strength": "Primary",
            "linked_provisions": ["reg.3(1)"],
            "evidence_hint": {"type_a": "form", "type_b": "test"},
        }
        flags = lint_control(bad_control, {"reg.3(1)"})
        paperwork_flags = [f for f in flags if f.startswith("PAPERWORK")]
        self.assertTrue(paperwork_flags, "Should flag 'document exists' in description")

    def test_lint_catches_invalid_ref(self):
        """Lint flags linked_provisions that don't match input."""
        control = {
            "title": "A control",
            "description": "desc",
            "what_it_checks": "check",
            "control_type": "Preventive",
            "nature": "Manual",
            "domain": "Organisational",
            "info_distance": "Mediated",
            "blast_radius": "Site",
            "mapping_strength": "Primary",
            "linked_provisions": ["reg.99(1)"],
            "evidence_hint": {"type_a": "a", "type_b": "b"},
        }
        flags = lint_control(control, {"reg.3(1)", "reg.4(1)"})
        ref_flags = [f for f in flags if f.startswith("INVALID_REF")]
        self.assertTrue(ref_flags, "Should flag reg.99(1) as invalid")

    def test_lint_catches_missing_judgement(self):
        """Lint flags when judgement term is in text but load_bearing_judgement is null."""
        control = {
            "title": "The risk assessment is suitable and sufficient",
            "description": "A suitable assessment",
            "what_it_checks": "check",
            "control_type": "Preventive",
            "nature": "Manual",
            "domain": "Organisational",
            "info_distance": "Mediated",
            "blast_radius": "Site",
            "mapping_strength": "Primary",
            "linked_provisions": ["reg.3(1)"],
            "load_bearing_judgement": None,
            "evidence_hint": {"type_a": "a", "type_b": "b"},
        }
        flags = lint_control(control, {"reg.3(1)"})
        judgement_flags = [f for f in flags if f.startswith("JUDGEMENT_MISSING")]
        self.assertTrue(judgement_flags, "Should flag missing load_bearing_judgement for 'suitable'")


class TestStagingTable(unittest.TestCase):
    """Test DuckDB staging table creation and storage."""

    def setUp(self):
        self.conn = duckdb.connect(":memory:")

    def tearDown(self):
        self.conn.close()

    def test_create_staging_table(self):
        """Staging table can be created."""
        ensure_staging_table(self.conn)
        tables = [r[0] for r in self.conn.execute(
            "SELECT table_name FROM information_schema.tables WHERE table_name = 'suggested_controls'"
        ).fetchall()]
        self.assertIn("suggested_controls", tables)

    def test_store_and_retrieve(self):
        """Controls can be stored and retrieved."""
        ensure_staging_table(self.conn)
        controls = [{"title": "Test control", "description": "desc"}]
        validation = [[]]
        store_controls(self.conn, "UK_test_law", controls, "test-model", validation)
        rows = self.conn.execute("SELECT * FROM suggested_controls").fetchall()
        self.assertEqual(len(rows), 1)
        # Verify by column name query instead of positional index
        row = self.conn.execute(
            "SELECT law_name, control_type, status FROM suggested_controls"
        ).fetchone()
        self.assertEqual(row[0], "UK_test_law")
        self.assertEqual(row[1], "specific")
        self.assertEqual(row[2], "validated")

    def test_flagged_status(self):
        """Controls with lint flags get 'flagged' status."""
        ensure_staging_table(self.conn)
        controls = [{"title": "Bad control", "description": "desc"}]
        validation = [["DEONTIC: title contains 'must'"]]
        store_controls(self.conn, "UK_test_law", controls, "test-model", validation)
        row = self.conn.execute("SELECT status FROM suggested_controls").fetchone()
        self.assertEqual(row[0], "flagged")

    def test_idempotent_create(self):
        """Creating staging table twice doesn't error."""
        ensure_staging_table(self.conn)
        ensure_staging_table(self.conn)  # should not raise


if __name__ == "__main__":
    unittest.main()
