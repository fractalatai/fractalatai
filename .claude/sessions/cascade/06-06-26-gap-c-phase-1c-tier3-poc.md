# Session: 2026-06-06 — Gap C Phase 1C: Tier 3 LLM Proof of Concept

## Context

**Meta-plan**: `.claude/plans/gap-c-tiered-resolution.md`
**Design doc**: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md` v0.4
**Prior**: Phase 1A complete (6,175 inherited, 76% precision), Phase 1B deferred (#36)

## Objective

Prove that an LLM can correctly distinguish duty **holders** from **recipients** in provisions where Tier 1 inheritance produced false positives. Use the 9 INCORRECT cases from the Tier 1 QA as the test set.

## The Pattern 2 Problem

Parent: "The employer shall provide training to employees"
- parse_v2 extracts: [Org: Employer, Ind: Employee]
- Tier 1 inherits both to child provision
- But: Employer is the **holder**, Employee is the **recipient**

The LLM needs to answer: "Given this parent text and these actors, which one HOLDS the duty?"

## Test Cases (from QA run 2)

9 provisions where Tier 1 inheritance was judged INCORRECT by Gemini.
Stored in `data/qa-results/inherited-qa-20260606-110728.json`.

## Results

- [x] Built Tier 3 prompt — HOLDER/RECIPIENT/BENEFICIARY/MENTIONED classification
- [x] Tested on 9 INCORRECT cases from Tier 1 QA
- [x] **8/9 cases correctly resolved** by Gemini 2.5 Flash

| Case | Inherited | LLM primary_holder | LLM recipient | Correct? |
|------|-----------|-------------------|---------------|----------|
| 1 | Person + 4 others | Ind: Person | Others: MENTIONED | Yes |
| 2 | Worker | None (recipient only) | Worker: RECIPIENT | Yes |
| 3 | Public | NDA (inferred) | Public: BENEFICIARY | Yes |
| 4 | Employee + Employer | Org: Employer | Employee: RECIPIENT | Yes |
| 5 | Worker | Org: Employer (inferred) | Worker: RECIPIENT | Yes |
| 6 | Worker | Org: Employer (inferred) | Worker: BENEFICIARY | Yes |
| 7 | Operator | Operator | — | Disagrees with QA |
| 8 | Worker | Org: Employer (inferred) | Worker: RECIPIENT | Yes |
| 9 | Employee + Employer | Org: Employer | Employee: BENEFICIARY | Yes |

### Key findings

1. **Holder/recipient distinction works cleanly** — the LLM never confused the two
2. **Can infer holders not in the actor list** — cases 5, 6, 8 correctly identified Employer even though only Worker was inherited
3. **Returns null when no holder exists** — case 2 correctly identified a definition, not a duty
4. **Recipient data is a natural byproduct** — captured without additional cost or prompting

### Discovery: recipient as a data model extension

The RECIPIENT/BENEFICIARY roles the LLM produces are valuable data we were discarding. Added to design doc as "Recipient Model" section:
- `recipient` column: who receives/benefits from the obligation
- `recipient_type`: protected_person / regulated_actor / informed_party / beneficiary
- Two distinct populations: governed duties protect people, government responsibilities regulate actors
- Tier 3 provisions self-select for this data (they failed Tier 1 because of multi-actor ambiguity)

## Next Steps

- [ ] Commit POC + design doc update
- [ ] Decision: integrate Tier 3 into enrich_single_law, or next session?
