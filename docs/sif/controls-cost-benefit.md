# Controls cost-benefit: where are we over- and under-investing in protection?

## The idea

Every organisation has a portfolio of risks and a portfolio of controls. The controls cost something — not always money, but always time, trouble, and effort. The risks each have a consequence distribution. If we can measure the control investment for each risk and plot it against the consequence severity, across a population of comparable risks, the scatter plot reveals where the organisation is over-investing (high effort, low consequence) and under-investing (low effort, high consequence).

This is not a new idea. Cost-benefit analysis of safety controls is standard in principle. What has been missing is a structured way to score control investment that distinguishes between control types, and a continuous measure of consequence that distinguishes between severity levels. The [PROPERTY × METHOD × EVENT classification](consequence-is-not-fixed.md) provides the first. The [severity distribution and P(SIF)](percentiles-and-distributions.md) provides the second.

## What to measure

The scatter plot has two axes:

**X-axis — consequence severity.** The unmitigated P(SIF) for each risk, calculated from the [energy-based severity distribution](energy-wheel.md). This is the inherent severity — what happens if all controls fail. It is a property of the energy source, not of the control portfolio. A 6-metre fall has an unmitigated P(SIF) ≈ 0.78 regardless of what nets or harnesses are in place.

**Y-axis — protective control investment.** The total operational burden of the protection (mitigation) controls applied to that risk. Not the prevention controls — those reduce the probability of the event and belong on a different analysis. Protection controls shift the [consequence distribution](consequence-is-not-fixed.md), so plotting them against consequence severity shows whether the investment in shifting the distribution is proportional to the severity being shifted.

Each point on the scatter is one risk — one hazard scenario at one location or in one task.

## Scoring control investment

The challenge is measuring "time, trouble, and effort" in a way that is comparable across different control types. Financial cost alone misses the point — a management control like a daily toolbox talk has a low procurement cost but a high recurring time cost across the workforce. A design control like prefabrication has a high upfront cost but near-zero ongoing burden.

### Not another ordinal scale

The temptation is to score each control on a 1–5 scale for time, trouble, and effort, then multiply the scores. This is a probability-impact grid for costs — the same tool this paper argues against. It has the same defects: range compression destroys information, multiplying ordinal ranks is mathematically meaningless, and the distance between "low" and "medium" is not the same as between "medium" and "high." If we reject ordinal scoring for consequence severity, we should reject it for cost.

The alternative is the same approach we use for everything else in this framework: **estimate the cost as a distribution, not a point.**

### Cost as a distribution

Control cost is uncertain. A safety net installation might cost £2,000 on a straightforward scaffold or £8,000 on a complex façade with restricted access. Annual inspection varies by site. The rescue plan requires re-training when staff turn over — which happens unpredictably. Point estimates hide this variation. Three percentile estimates capture it:

- **P10** (optimistic): the cost under good conditions — experienced crew, standard installation, low turnover
- **P50** (typical): the most likely cost in normal operation
- **P90** (pessimistic): the cost under difficult conditions — awkward access, high turnover, specialist equipment needed

These three numbers define a [metalog distribution](metalog-in-plain-english.md) for the control's cost — exactly as three severity estimates define a metalog for consequence. The same mathematics, the same elicitation technique, the same [calibration training](calibrated-estimation.md) to improve the estimates.

The cost should be expressed as **annualised operational burden** — the recurring cost per year of maintaining the control in effective working order. This normalises across control types:

- **Design controls**: high upfront capital cost amortised over the asset life, plus low annual maintenance. A £50,000 prefabrication investment over a 10-year asset life = £5,000/year + £500/year maintenance → annualised P50 ≈ £5,500.
- **Engineer controls**: moderate installation amortised, plus regular inspection/testing cycles. A safety net at £4,000 replaced every 5 years + annual inspection at £300 → annualised P50 ≈ £1,100.
- **Management controls**: negligible capital, high recurring cost in person-hours. A rescue plan requiring 4 hours of training × 12 people × 2 sessions/year at a loaded labour rate → annualised P50 ≈ £4,800. Staff turnover (P90 scenario) could push this to £8,000+.

The unit is £/year (or $/year, or person-hours/year — whichever the organisation tracks). This is standard project management cost estimation. Savage's *The Flaw of Averages* makes the case that every cost estimate in an organisation should be a distribution, not a point — and the metalog is the tool to do it. Hubbard's *How to Measure Anything* provides the calibration method to ensure the distributions are honest.

### The control cost library

Estimating cost distributions for every control from scratch is impractical. The solution is a **library of control archetypes** — each with default cost distributions that the user adjusts to their context.

The library is structured by the PROPERTY × METHOD matrix:

| | Design | Engineer | Management |
| --- | --- | --- | --- |
| **Protection** | Lower-energy system design | Safety net, hard hat, arc flash PPE, fall harness | Emergency response plan, first aid training |
| **Exposure duration** | Prefabrication, automation | Interlocked access gates, timed permits | Permit duration limits, shift rotation |
| **Chance per unit exposure** | Inherently stable design | Guardrails, LOTO, machine guarding | Training, toolbox talks, supervision |

Each archetype entry contains:

```
Control:        Safety net (fall protection)
PROPERTY:       Engineer
METHOD:         Protection
EVENT coverage: Normal + Abnormal
Default cost (annualised, £/year):
  P10:    600    (simple scaffold, easy access, long replacement cycle)
  P50:  1,100    (typical installation, 5-year replacement, annual inspection)
  P90:  3,200    (complex façade, frequent repositioning, specialist rigger)
Sources:        FASET guidance, HSE INDG401, supplier quotations 2024-25
```

The user's workflow:

1. **Select** the control archetype from the library.
2. **Review** the default P10/P50/P90 — do they match this site, this task, this context?
3. **Adjust** where needed. A confined-space rescue plan at an offshore platform costs more than one at a ground-level factory. The user shifts the percentiles to reflect their reality.
4. **The system** fits the metalog and carries the cost distribution forward into the analysis.

The defaults provide a starting point grounded in published data and industry benchmarks. The user provides the local adjustment. This is the same pattern as the [severity calibration curves](energy-wheel.md) — population-level data as the prior, site-level knowledge as the update.

**Building the library with AI.** The initial library can be constructed from published cost data (HSE cost-benefit guidance, LOPA reliability data, supplier catalogues, insurance actuarial tables) and augmented by a large language model extracting cost ranges from safety industry literature, procurement records, and maintenance logs. The LLM's role is data assembly, not cost estimation — it gathers and structures published figures into the P10/P50/P90 format. Human review validates the defaults before they enter the library. Over time, the library improves as organisations feed back their actual costs, narrowing the distributions with real data — the same [Bayesian updating](bayesian-priors.md) pattern used for severity priors.

### Aggregating per risk

For a given risk, the total protection cost is the sum of the annualised cost distributions of all protection controls applied to that risk. Because costs are distributions, not points, they compose via [SIPmath](monte-carlo-and-swiss-cheese.md): generate 10,000 trials from each control's cost metalog, sum element-wise, and the result is the total cost distribution.

The output is not a single number but a distribution:
- P50 total protection cost = the typical annual burden
- P90 total protection cost = the burden in a bad year (multiple replacements, high turnover, difficult access)
- The spread (P90 − P10) = how uncertain the cost estimate is

### Example: fall from 6-metre scaffold

| Protection control | P10 (£/yr) | P50 (£/yr) | P90 (£/yr) |
| --- | --- | --- | --- |
| Safety net (Engineer × Protection) | 600 | 1,100 | 3,200 |
| Fall harness programme (Engineer × Protection) | 400 | 800 | 1,500 |
| Hard hat (Engineer × Protection) | 50 | 100 | 200 |
| Rescue plan (Management × Protection) | 2,400 | 4,800 | 8,500 |

Summing the distributions (via SIPmath, not by adding the P50s):
- **Total protection cost P50 ≈ £6,800/year**
- **Total protection cost P90 ≈ £12,500/year**

The rescue plan dominates — not because it is expensive to procure, but because it requires ongoing training, coordination across multiple roles, and must function during both normal and abnormal/emergency conditions. The distribution also reveals that cost uncertainty is driven almost entirely by the rescue plan: in the P90 scenario (high staff turnover, complex site), it alone accounts for £8,500 of the £12,500 total. This is actionable information — if the organisation wants to reduce cost uncertainty, stabilising the rescue team (reducing turnover) is the highest-leverage intervention.

## The scatter plot

Plot all comparable risks on the same chart:

```
Protection cost
(£k/yr, P50)
       ↑
   15  │                              × Confined space entry
       │                    × Scaffold 6m
   12  │         × Crane lift
       │                         × Electrical HV
    9  │    × Scaffold 3m
       │              × Mobile plant
    6  │  × Ladder work
       │         × Vehicle (occupant)
    3  │
       │
    0  │──────────────────────────────────→
       0    0.2    0.4    0.6    0.8    1.0
                Unmitigated P(SIF)
```

Each point is plotted at its P50 cost, but carries its full cost distribution. An error bar from P10 to P90 shows cost uncertainty — wide bars mean the cost is volatile and worth investigating. The x-axis uses the unmitigated P(SIF) from the [energy model](energy-wheel.md), which is deterministic for a given scenario.

### Reading the plot

**The expected pattern** is a positive correlation: higher-severity risks should attract more protection investment. The trend line through the population represents the organisation's implicit standard — the average investment per unit of consequence severity.

**Above the trend line** — over-invested relative to peers. The protection effort is higher than comparable risks with similar severity. This is not necessarily wrong — the risk may warrant extra protection. But it raises the question: could some of this effort be redirected to under-protected risks?

**Below the trend line** — under-invested relative to peers. The protection effort is lower than comparable risks with similar severity. These are the risks that should concern the safety team most: the consequence distribution is severe and the investment in shifting it is below the organisation's own norm.

**Far right, near the x-axis** — high severity, minimal protection. These are the most dangerous outliers: risks where the unmitigated P(SIF) is high and the organisation has invested little in consequence reduction. They may be relying entirely on prevention controls (reducing the event probability) — which is a viable strategy only if those prevention controls are highly reliable.

**Far left, high on the y-axis** — low severity, heavy protection. Over-engineered. The consequence distribution is mild and the organisation is spending disproportionate effort protecting against it. Common with legacy controls that were installed after a high-profile incident and never re-evaluated.

## Adding the residual dimension

The scatter above shows investment vs inherent severity, but it does not show whether the investment is *working*. A risk with high protection investment and high unmitigated P(SIF) could have a residual P(SIF) of 0.02 (controls are effective) or 0.60 (controls are ineffective despite the effort).

Encode the residual P(SIF) as the point colour or size:

| Residual P(SIF) | Colour | Meaning |
| --- | --- | --- |
| < 0.10 | Green | Well-controlled — protection is working |
| 0.10–0.50 | Amber | Partially controlled — some SIF potential remains |
| ≥ 0.50 | Red | Inadequately controlled — protection investment is not shifting the distribution enough |

A red point high on the y-axis is the worst case: high effort, high residual risk. The controls are consuming resources without adequately shifting the consequence distribution. This points to either the wrong type of controls (e.g., management controls where engineering controls are needed), degraded controls (installed but not maintained), or controls that don't match the energy type (a hard hat against an electrical hazard).

A green point low on the y-axis is the best case: low effort, low residual risk. Either the inherent severity is low (left side of the x-axis) or a small number of highly effective controls is doing the job efficiently.

## The efficiency frontier

Across the population of risks, some will achieve more P(SIF) reduction per unit of investment than others. These form the **efficiency frontier** — the boundary of what is achievable at each investment level.

Plot a second chart:

- **X-axis**: ΔP(SIF) = unmitigated P(SIF) − residual P(SIF). This is the risk reduction achieved.
- **Y-axis**: protection investment (the same weighted score).

The slope from the origin to each point is the **cost per unit of risk reduction**. The steepest slopes (most reduction per unit of effort) form the frontier. Points well above the frontier are achieving the same reduction at much higher cost — candidates for control rationalisation.

```
Protection cost
(£k/yr, P50)
       ↑
   15  │  ×                      ← High cost, moderate reduction
       │       ×
   12  │            ×
       │    ×            ×       ← Efficiency frontier
    9  │         ×            ×
       │              ×
    6  │    ×              ×     ← Low cost, good reduction
       │
    0  │──────────────────────────→
       0    0.2    0.4    0.6    0.8
            ΔP(SIF) achieved
```

Points far above the frontier → controls are expensive but ineffective. Review the control portfolio: are the controls matched to the energy type? Are they maintained? Are they the right METHOD (protection vs prevention)?

Points on or near the frontier → efficient. These represent the organisation's best practice for protection investment.

Points that cluster in the bottom-left → low investment, low reduction. Acceptable if the unmitigated P(SIF) is also low. Concerning if it is high — these are under-protected risks hiding behind an apparent low cost.

## Comparable populations

The scatter is only meaningful within a population of comparable risks. "Comparable" means risks where the cost structures and severity scales are similar enough that the trend line represents a real benchmark.

Sensible population groupings:

| Grouping | Rationale |
| --- | --- |
| By energy type | All gravity risks, all electrical risks — the severity distributions and effective controls are specific to the energy type |
| By ICECI mechanism | All falls, all struck-by, all caught-in — aligns with the [SIF classifier](../papers/from-pigs-to-probability-curves.md) Stage 1 output |
| By site or sector | Controls costs vary by location (offshore vs office) and industry |
| By STKY hazard | Hallowell's 13 high-energy categories — each has a characteristic control portfolio |

Mixing energy types on one chart will produce noise: the cost of protecting against a gravity hazard (nets, harnesses) is structurally different from protecting against a chemical hazard (ventilation, PPE, decontamination). Within one energy type, the trade-offs are comparable and the outliers are meaningful.

## What this does not capture

**Prevention controls.** This analysis focuses on protection (mitigation) — controls that shift the consequence distribution. Prevention controls (exposure duration, chance per unit exposure) reduce the probability of the event occurring. They belong on a separate analysis: prevention investment vs event frequency. The two analyses together give the full picture of the control portfolio, but mixing them on one chart conflates the two sides of the bowtie.

**Control reliability.** The cost distribution measures the operational burden of maintaining the control, not whether it actually works. A high-cost control can still be ineffective if it is the wrong type for the hazard, or if it is systematically bypassed during [abnormal or emergency operations](consequence-is-not-fixed.md). The residual P(SIF) colour-coding partially addresses this — a high-cost, high-residual point signals ineffectiveness — but root cause analysis is needed to understand why.

**Interactions between controls.** The cost distributions sum across controls independently. In practice, controls interact: a rescue plan (management) is more effective when combined with a safety net (engineer) because the net limits fall distance and the rescue plan handles the cases where the net partially fails. The [Monte Carlo simulation](monte-carlo-and-swiss-cheese.md) captures these interactions in the residual P(SIF), but the cost side does not model them — it treats each control's cost as independent. For most purposes this is acceptable: the cost of maintaining a rescue plan does not change much whether or not a net is also present. But there are cases where shared infrastructure (a common anchor system serving both nets and harnesses) reduces the per-control cost, and the library entry would need site-specific adjustment.

**Non-safety benefits.** Some controls have value beyond safety. [Operational efficiency measures](consequence-is-not-fixed.md) that reduce exposure duration — faster task execution, automation, prefabrication — also reduce project cost and schedule. Their safety benefit is real but their cost is shared with other objectives. The scatter may show these as over-invested for their safety benefit alone, when in fact they are paying for themselves on the operations side. A note in the risk register flagging shared-benefit controls would prevent them from being rationalised out of the safety portfolio.

## Operationalising it

### Data required

For each risk in the register:

1. **Hazard description** — energy type, mechanism, source properties (height, voltage, mass, etc.)
2. **Unmitigated P(SIF)** — from the [energy-based severity model](energy-wheel.md) or expert [percentile estimates](percentiles-and-distributions.md)
3. **List of protection controls** — each tagged with PROPERTY (design/engineer/management) and EVENT coverage (normal/abnormal/emergency)
4. **Cost distribution per control** — P10/P50/P90 annualised cost, starting from the library default and adjusted to local context
5. **Residual P(SIF)** — from the [simulator](monte-carlo-and-swiss-cheese.md) with control effectiveness distributions applied

Items 1–3 should already exist in any risk register that tags controls by type. Item 4 uses the control library as its starting point — the user reviews and adjusts the defaults rather than estimating from scratch. Item 5 requires the metalog/SIPmath engine but can be approximated by expert estimate of residual severity quantiles.

### Process

1. **Scope**: select a comparable population (e.g., all gravity risks across three sites).
2. **Cost**: for each risk, select controls from the library, adjust cost distributions to context, and aggregate via SIPmath.
3. **Plot**: scatter P50 cost vs unmitigated P(SIF), coloured by residual P(SIF). Add P10–P90 error bars on the y-axis to show cost uncertainty.
4. **Fit**: draw the trend line through the population.
5. **Identify**: flag risks more than one standard deviation above or below the trend.
6. **Investigate**: for each outlier, ask:
   - Over-invested: are any controls redundant, legacy, or delivering diminishing marginal return?
   - Under-invested: what protection controls are missing, and what would they cost? The library provides the default estimate immediately.
   - High cost / high residual (red, high on y-axis): why aren't the controls working? Wrong type? Degraded? Bypassed during non-routine work?
   - Wide error bars: cost is volatile — what is driving the uncertainty? Can it be stabilised?
7. **Rebalance**: propose reallocation — shift investment from over-invested risks to under-invested risks, keeping total portfolio cost constant while reducing total portfolio P(SIF). The SIPmath engine models the portfolio-level effect of the reallocation.

### The rebalancing argument

This is where the analysis becomes actionable. Safety budgets are finite. The scatter shows the organisation its own allocation pattern — where it has concentrated protection effort and where it has not. If the trend reveals that legacy controls on low-severity risks are consuming 40% of the protection budget while high-severity risks are under-protected, the rebalancing case makes itself.

The argument is not "spend more on safety." The argument is "spend the same amount, differently." The scatter provides the evidence; the [simulator](monte-carlo-and-swiss-cheese.md) provides the modelling of what reallocation would achieve; the P(SIF) numbers provide the before-and-after comparison.

## Sources

- Hallowell, M.R. (2024). *Energy-Based Safety*. CRC Press/Routledge.
- Hubbard, D.W. (2010). *How to Measure Anything*, 2nd ed. Wiley. — Calibrated estimation for cost distributions; the case that any uncertain quantity can be estimated as a range.
- Savage, S.L. (2009). *The Flaw of Averages*. Wiley. — Every cost estimate should be a distribution, not a point; SIPmath composition for aggregating uncertain costs.
- Keelin, T.W. (2016). The Metalog Distributions. *Decision Analysis*, 13(4), 243–277. — Three percentile estimates → closed-form distribution, applicable to costs as well as severity.
- CCPS/AIChE. *Layer of Protection Analysis: Simplified Process Risk Assessment*. — LOPA methodology for control reliability and cost-effectiveness.
- HSE (UK). *Reducing Risks, Protecting People (R2P2)*. — ALARP and cost-benefit framework for safety controls.
- HSE (UK). *Cost Benefit Analysis (CBA) Checklist*. — Guidance on annualising control costs for ALARP demonstrations.
