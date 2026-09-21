# Monte Carlo simulation and the quantified Swiss cheese model

## What is Monte Carlo simulation?

Monte Carlo simulation is a way of answering "what could happen?" by running a scenario thousands of times with random variation, then looking at the spread of results.

Think of it as a thought experiment made rigorous. If a worker falls from a 6-metre scaffold, we cannot predict exactly what will happen — there are too many variables (angle of fall, body part struck, exact surface condition, clothing, fitness). But we *can* describe each variable as a range (a [distribution](percentiles-and-distributions.md)), then ask a computer to "roll the dice" 10,000 times and record every outcome.

After 10,000 trials:
- Some trials produce a fatality
- Some produce a serious injury
- Some produce only medical treatment
- A few produce minor injuries

The fraction of trials that produce a serious injury or fatality *is* P(SIF). If 7,800 out of 10,000 trials result in serious injury or worse, P(SIF) = 0.78. No formula needed — just counting.

## Why 10,000?

At 1,000 trials, the answer might fluctuate by a few percentage points each time you run it. At 10,000, it stabilises. At 100,000, the extra precision is rarely worth the extra time. 10,000 is the conventional sweet spot: stable enough for decisions, fast enough to run in a browser.

## The Swiss cheese model — from poster to calculator

James Reason's Swiss cheese model is familiar to anyone in safety: defences are like slices of cheese, each with holes. An accident happens when the holes line up and a hazard passes through every slice.

The model is powerful as a concept but useless as a calculator. It does not tell you *how big* the holes are, *how often* they line up, or *what happens* when they do.

The SIPmath approach quantifies each slice. Every mitigation — a safety net, a harness, a hard hat — has an effectiveness distribution expressed as three percentile estimates (see [metalog](metalog-in-plain-english.md)):

| Mitigation | P10 | P50 | P90 |
| --- | --- | --- | --- |
| Safety net | 0.75 | 0.90 | 0.97 |
| Fall harness (worn and attached) | 0.80 | 0.92 | 0.98 |
| Hard hat | 0.40 | 0.60 | 0.80 |

An effectiveness of 0.90 means the mitigation absorbs 90% of the severity. But it is not always 0.90 — sometimes the net is poorly rigged (0.60), sometimes the harness catches perfectly (0.98). The distribution captures this range.

## How mitigations combine

For each of the 10,000 Monte Carlo trials, the system:

1. Draws a severity from the unmitigated severity distribution (e.g., 4,400 J fall)
2. Draws an effectiveness for each mitigation from its distribution
3. Calculates: **residual severity = severity × (1 − net effectiveness) × (1 − harness effectiveness)**

In trial #47 where the net is poorly rigged (effectiveness = 0.30) and the harness is unclipped (effectiveness = 0):
> residual = severity × 0.70 × 1.00 = 70% of unmitigated severity — still dangerous

In trial #8,002 where both work well (net = 0.95, harness = 0.97):
> residual = severity × 0.05 × 0.03 = 0.15% of unmitigated severity — negligible

P(SIF) after mitigations = the fraction of 10,000 trials where residual severity still exceeds the SIF threshold. For two good barriers on a 6-metre fall, this drops from 0.78 to roughly 0.02.

## The independence problem

There is a hidden danger in simple multiplication. If the safety net was last inspected by the same person who checks the harnesses, and that person has been cutting corners, *both* barriers are degraded at the same time. They are not independent.

The system handles this by linking the random draws for correlated mitigations. In trials where one barrier is degraded due to a shared cause (deferred maintenance, schedule pressure, extreme weather), the other is degraded too. This is more realistic than assuming each barrier fails independently, which is the dangerous assumption most qualitative risk assessments make without stating it.

## What this gives a safety professional

Instead of a Swiss cheese poster on the wall and a gut feeling about whether the controls are "adequate," the professional gets:

- **A number**: P(SIF) drops from 0.78 to 0.02 with two barriers in place
- **The marginal value of each control**: removing the net raises P(SIF) to 0.15; removing the harness raises it to 0.08 — the net matters more for this scenario
- **Honest failure modes**: in 200 out of 10,000 trials, despite both barriers, the outcome is still serious or fatal — these are the correlated failure cases where both controls are degraded simultaneously
- **A decision basis**: is P(SIF) = 0.02 acceptable? That is a policy decision, not a technical one. But at least the decision is being made on a real number rather than a colour
