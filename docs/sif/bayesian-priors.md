# Bayesian priors: what happens when information is missing

## The problem

An incident report says: "Worker fell from ladder." That is all. No height. No ladder type. No description of the landing surface.

A probability-impact grid handles this by asking the assessor to guess. Different assessors will guess differently. The assessment becomes a measure of the assessor's imagination, not the hazard.

## What a prior is

A Bayesian prior is a starting estimate based on what we already know about similar situations — before seeing any specifics of this particular case.

For "fell from ladder," the prior is built from OSHA data on all ladder falls. That data tells us:
- Most ladder falls are from step ladders at 1–2 metres
- Extension ladder falls tend to be from 3–6 metres
- The overall distribution of ladder fall heights has a median around 2.5 metres

This distribution is the prior. It is not a guess — it is the actual statistical pattern from thousands of recorded ladder falls.

## How clues narrow the prior

Each piece of information in the narrative narrows the prior towards a more specific estimate:

| What the report says | Effect on the estimate |
| --- | --- |
| "Fell from ladder" (no detail) | Use the full prior: median ~2.5 m, wide range |
| "Fell from extension ladder" | Narrow to extension ladder heights: median ~4 m |
| "Fell from top of extension ladder" | Narrow further: likely 4–6 m |
| "Fell from 6-metre scaffold" | Prior collapses: height = 6 m (near-certain) |

The more the reporter tells us, the tighter the estimate becomes. With no detail, P(SIF) might be 0.35 (reflecting the wide range of possible ladder heights). With "top of extension ladder," P(SIF) might be 0.55. With "6-metre scaffold," P(SIF) is 0.78.

## The feedback loop

This creates a natural incentive to write better incident reports. When the system returns "P(SIF) = 0.35, but fall height was estimated from prior — specify actual height for a more precise classification," the reporter learns that height matters. Over time, reports get more specific where it counts, and the system gets more accurate.

No probability-impact grid has ever improved the quality of incident reporting. The grid takes whatever the report says and maps it to a cell. There is no feedback mechanism, no way for the system to say "this information would change the assessment."

## Why "I don't know" is an honest answer

The Bayesian approach makes uncertainty visible. A P(SIF) of 0.35 based on a vague report is flagged differently from a P(SIF) of 0.35 based on specific measurements. The first carries high uncertainty — more information could move it substantially. The second is a confident estimate. The system distinguishes between them, and can calculate the [value of gathering more information](calibrated-estimation.md) before making a decision.

This is the opposite of a risk matrix, where "I don't know" typically results in the assessor picking the middle cell and moving on.
