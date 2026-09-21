# Calibrated estimation

## The overconfidence problem

When asked to give a range estimate — "give me a range you are 90% confident contains the true answer" — most people give ranges that are far too narrow. Studies by Douglas Hubbard and others consistently show that untrained estimators' 90% confidence intervals contain the true value only about 50% of the time. People think they know more precisely than they do.

This matters because the [metalog distribution](metalog-in-plain-english.md) and [Monte Carlo simulation](monte-carlo-and-swiss-cheese.md) are only as good as the estimates that feed them. If a safety professional says "P10 = medical treatment, P90 = serious injury" when the real P90 is fatality, the system will underestimate P(SIF).

## What calibration training does

Calibration training teaches people to give honest ranges — not wider for the sake of it, but accurately reflecting what they do and do not know. It typically takes half a day and involves:

1. **Baseline test**: answer a series of questions with 90% confidence intervals (e.g., "what is the height of the Eiffel Tower?"). Score how many true answers fall inside your ranges.
2. **Feedback**: most people score around 50% — their ranges are too narrow half the time. Seeing this concretely is the key moment.
3. **Practise rounds**: repeated estimation with immediate feedback. People learn to widen their ranges where they are genuinely uncertain and keep them tight where they have real knowledge.
4. **Re-test**: after training, most people score 80–90% — close to the target.

The result is not that everyone gives wide, vague ranges. A calibrated estimator who knows ladder falls well will give tight, accurate ranges for ladder falls and wider ranges for chemical exposures they know less about. The ranges reflect actual knowledge, not false confidence.

## Why this matters for SIF assessment

A calibrated safety professional providing P10/P50/P90 estimates for severity produces a [distribution](percentiles-and-distributions.md) that genuinely reflects the range of possible outcomes. An uncalibrated professional tends to compress the range — underestimating how bad the worst case could be and overestimating how good the best case might be. In SIF terms, this means underestimating P(SIF).

The system still works with uncalibrated inputs — the structure of asking for three percentiles imposes more discipline than a risk matrix. But calibrated inputs are materially better, and the training investment is small relative to the improvement.

## The value of information

Calibrated estimation connects to a powerful concept: the **expected value of information**. If P(SIF) = 0.45 and the decision threshold is 0.50, the assessment is close to a tipping point. Gathering more information (measuring the actual fall height, checking the surface material) could move the estimate across the threshold and change the decision.

The system can calculate how much that information is worth: if measuring the fall height would change the decision in 30% of cases, and each wrong decision costs £50,000 in investigation time or unmitigated risk, then the measurement is worth up to £15,000. This is not hypothetical arithmetic — it is standard decision theory, and it becomes practical once you have a probability model underneath.

A risk matrix cannot do this calculation because it has no model. It has coloured cells and arrows between them.

## Further reading

- Hubbard, D.W. (2010). *How to Measure Anything*, 2nd ed. Wiley. Chapters 5–7 cover calibration training in detail.
- Hubbard, D.W. (2009). *The Failure of Risk Management*. Wiley. Chapter on why expert judgement fails without calibration.
