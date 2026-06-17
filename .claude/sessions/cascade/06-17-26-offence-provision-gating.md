# Session: Offence Provision Gating (PENDING)

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

## Scope

1. Add `Offence` to the purpose gate skip list in `should_skip_drrp()`
2. Verify no false negatives — check if any genuine duties have Offence as primary purpose
3. Update gold standard to mark offence provisions as `none`
4. Re-benchmark

## Key files

- `crates/fractalaw-core/src/taxa/mod.rs` — `should_skip_drrp()` gate
- `crates/fractalaw-core/src/taxa/purpose.rs` — Offence purpose classifier
- `crates/fractalaw-core/src/taxa/duty_patterns_offence.rs` — offence-as-duty patterns
