# Percentiles and distributions

## The problem with single numbers

If someone asks "how bad could a 6-metre fall be?", the honest answer is not a single word like "serious." It is a range: most of the time a serious injury, sometimes a fatality, occasionally just medical treatment. The outcome depends on how the person lands, what they land on, which body part takes the impact, and dozens of other variables.

A single number (or a single cell on a risk matrix) hides this range. A *distribution* shows it.

## What a distribution is

A distribution is a picture of all the possible outcomes and how likely each one is. Think of it as a hill:

- The peak of the hill is the most common outcome
- The left slope shows better-than-typical outcomes (less severe)
- The right slope shows worse-than-typical outcomes (more severe)
- The area under any part of the hill tells you how likely those outcomes are

For a 6-metre fall onto concrete, the hill is centred around "serious injury" but has a substantial right tail reaching into "fatality." That tail is the SIF potential.

## P10, P50, P90 — three points that define the hill

Rather than drawing the whole hill, we describe it with three numbers:

| Notation | Meaning | Plain English |
| --- | --- | --- |
| **P10** | 10th percentile | The optimistic end — only 10% of outcomes are less severe than this |
| **P50** | 50th percentile (median) | The middle outcome — half are better, half are worse |
| **P90** | 90th percentile | The pessimistic end — only 10% of outcomes are more severe than this |

For our 6-metre fall:
- P10 = medical treatment (the lucky outcomes)
- P50 = serious injury (the typical outcome)
- P90 = fatality (the unlucky outcomes)

These three numbers are enough to reconstruct the full distribution using a [metalog](metalog-in-plain-english.md). A safety professional or historical data provides these estimates; the mathematics fills in everything between and beyond them.

## CDF — the cumulative distribution function

The CDF answers one question: **what is the probability that the outcome is at or below a given severity level?**

For the 6-metre fall, the CDF might say:
- P(severity ≤ first aid) = 0.05 — only a 5% chance of getting away with first aid
- P(severity ≤ medical treatment) = 0.15 — 15% chance of medical treatment or less
- P(severity ≤ serious injury) = 0.78 — 78% chance of serious injury or less

That last number means there is a 22% chance of something *worse* than serious injury — i.e., fatality. And P(SIF), the probability of serious injury *or worse*, is 1 − 0.22... wait, let's be precise. P(SIF) = P(severity ≥ serious injury) = 1 − P(severity < serious injury) = 1 − 0.15 = 0.85. The exact number depends on the fitted distribution, but the principle is: P(SIF) is read directly from the CDF. No guesswork, no committee debate, no coloured cells.

## Why this is better than a risk matrix

A risk matrix forces the 6-metre fall into one cell — say, "high severity / possible." It cannot express that the same event has a 10% chance of being only medical treatment and a 22% chance of being fatal. The distribution captures the full range. When you add a safety control, the distribution shifts — and you can see exactly how much it shifted and where the residual risk lies. The matrix can only be reassessed from scratch.
