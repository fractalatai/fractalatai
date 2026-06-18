# Session: Offence Provision Gating

## Context

**Prior session**: `.claude/sessions/cascade/06-11-26-drrp-qa-plan.md`
**Trigger**: 38 benchmark provisions gold-labelled as Duty are actually offence/penalty sections with no modal verb. These are not DRRP — they describe consequences of non-compliance, not obligations.

## Problem

Offence provisions like:
- "A person who contravenes the requirements of this section is guilty of an offence"
- "A person guilty of an offence under this section shall be liable on summary conviction to a fine"
- "It is an offence to intentionally obstruct any person acting in the execution of these Regulations"

These have NO modal verb (no shall/must/may). They describe criminal liability, not duties. The current pipeline correctly returns `drrp_types = []` for most of them. The gold standard incorrectly labels them as Duty.

## Two issues

### 1. Gold standard: offence provisions should not be Duty

The LLM classified "a person... is guilty of an offence" as Duty because there's an implicit obligation (don't contravene). But in the DRRP model, the obligation is in the section that creates the requirement — the offence section just attaches a penalty to non-compliance. Fix in gold standard session.

### 2. Pipeline: should offence provisions be gated?

The purpose classifier already has an `Offence` purpose. The `duty_patterns_offence.rs` module matches offence-as-duty patterns (P4 fix added penalty rejection). But some offence provisions still leak through as Duty when they match actor + modal patterns in surrounding text.

**Options:**
- A) Gate offence-primary provisions like we gate Enactment/Amendment — skip DRRP entirely
- B) Keep current P4 penalty rejection logic — offence provisions with "shall be liable" are already rejected
- C) Add offence provisions to the purpose gate skip list

Option A is cleanest. Offence provisions are structurally similar to Enactment — they describe consequences, not obligations. The duty is in the parent/sibling section.

## Broader problem: purpose classifier mislabelling (from gold correction session)

The FP analysis in the gold correction session revealed 86 regex false positives where gold=none but the pipeline says Obligation/Liberty. Root cause: the purpose classifier labels these as `Process+Rule` when they are actually:

- **Legal fictions** (52): "shall be treated/deemed/construed as", "Nothing in this section shall", "shall apply as if". These use "shall" in the interpretive sense, not the obligation sense. Purpose should be Interpretation, not Process+Rule.
- **Service method** (10): "notice may be served/sent/given by electronic means". Subordinate procedural detail.
- **Notice detail** (10): "notice must contain/specify/be in writing". Form requirements.
- **Offence** (9): "It is an offence for any person to". Already in scope of this session.
- **Scope extension** (5): "This article applies to...". Purpose should be Application+Scope, not Process+Rule.

### Fix approach: tighten the gate, not just add offence

The purpose gate currently allows `Process+Rule` provisions through unconditionally. But `Process+Rule` is too broad — it covers both substantive obligations AND procedural/interpretive provisions. 

**Principle**: Interpretation, offence, and similar structural sections are very unlikely to have DRRP. They should require a HIGH BAR before being allowed through — like a regex match on a canonical obligation pattern (actor + "shall/must" in subject position).

**Implementation**:
1. Gate `Offence`-primary provisions — skip DRRP unless a strong canonical actor+modal pattern is present
2. Tighten the `Process+Rule` gate for provisions that contain legal fiction language ("shall be treated/deemed/construed as")
3. The actor+modal anchor check from governed v2 is the right "high bar" — if governed v2 wouldn't match, the provision is likely structural

## Scope (revised)

1. Add `Offence` to the purpose gate skip list (with actor override, same as other gates)
2. Add legal fiction rejection: "shall be treated/deemed/construed/read as" provisions classified as Process+Rule should require a stronger signal (governed v2 anchor match) before producing DRRP
3. Verify no false negatives — check genuine duties aren't blocked
4. Update gold standard for offence provisions → none
5. Re-benchmark
6. Classifier retraining with none examples — fractalaw/fractalaw#39

## Key files

- `crates/fractalaw-core/src/taxa/mod.rs` — `should_skip_drrp()` gate
- `crates/fractalaw-core/src/taxa/purpose.rs` — Offence purpose classifier
- `crates/fractalaw-core/src/taxa/duty_patterns_offence.rs` — offence-as-duty patterns
- `crates/fractalaw-core/src/taxa/duty_type.rs` — `classify()` pattern cascade
