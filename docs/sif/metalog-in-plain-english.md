# The metalog distribution in plain English

## The old way: pick a shape, then fit it

Traditional statistics asks you to choose a distribution shape first — normal (bell curve), lognormal (skewed right), beta (bounded), Weibull, and so on. Then you fit that shape to your data. If you pick the wrong shape, your answers are wrong. Picking the right shape requires statistical training.

## The metalog way: give me your estimates, I'll give you the curve

The metalog distribution, published by Tom Keelin in 2016, works backwards. Instead of starting with a shape, it starts with what you already know: your percentile estimates.

You provide three numbers:
- **P10**: the value below which only 10% of outcomes fall (the optimistic end)
- **P50**: the median — the middle outcome
- **P90**: the value below which 90% of outcomes fall (the pessimistic end)

The metalog turns these three points into a smooth, continuous probability curve. No shape selection. No statistical fitting. Three numbers in, full distribution out.

## Why three numbers work

Three percentile estimates capture three properties of the distribution:

1. **Where is the centre?** → the P50 (median) tells you this
2. **How spread out is it?** → the gap between P10 and P90 tells you this
3. **Is it symmetric or skewed?** → whether P50 sits in the middle of P10 and P90, or closer to one end

A symmetric distribution (P50 midway between P10 and P90) says outcomes are equally likely to be better or worse than the median. A skewed distribution (P50 closer to P10) says the worst outcomes, while less common, are disproportionately bad — which is exactly what we see with serious injuries and fatalities.

## An example

For a 6-metre fall onto concrete, a safety professional (or historical data) provides:
- P10 = medical treatment (severity score 1.5)
- P50 = serious injury (severity score 3.0)
- P90 = fatality (severity score 6.0)

The metalog fits a curve through these three points. From that curve we can read off any probability we need — most importantly, P(SIF) = P(severity ≥ 3.0), which turns out to be approximately 0.78.

If we had more data points (P25, P75, etc.), the metalog can use those too — more points give a more precise curve. But three is enough for practical use, and three is the number of estimates a human can comfortably provide in a structured interview.

## Why this matters for safety

The metalog removes the statistical barrier that kept Monte Carlo simulation out of reach for most safety teams. You do not need to know what a "lognormal" is. You do not need to fit parameters. You need to answer three questions:

1. What is the best realistic outcome? (P10)
2. What is the most likely outcome? (P50)
3. What is the worst realistic outcome? (P90)

Safety professionals answer these kinds of questions every day. The metalog turns their answers into the mathematics that powers the [simulator](monte-carlo-and-swiss-cheese.md) and the [classifier](../papers/from-pigs-to-probability-curves.md).

## Further reading

- Keelin, T.W. (2016). The Metalog Distributions. *Decision Analysis*, 13(4), 243–277.
- ProbabilityManagement.org — the non-profit promoting metalog and SIPmath tools
