# Risk needs a reference

## The incomplete statement

"There is a 1-in-1,000 chance of death."

This statement has two of the three elements a risk statement requires:

1. **Chance**: 1 in 1,000
2. **Consequence**: death
3. **Reference**: *missing*

One in 1,000 per what? Per flight? Per year of flying? Per mile flown? Per lifetime? These are wildly different risk levels. A 1-in-1,000 chance of death per flight means you will almost certainly die if you fly regularly. A 1-in-1,000 chance of death per lifetime from flying is a background risk most people would not think about. The number is identical. The risk is not. The reference — measured over what exposure, for what population, across what scope — is what gives the number meaning.

Most probability-impact grids do not define the reference. "Likelihood: possible" — possible per shift? Per year? Per career? Per facility? The ambiguity is not a minor formatting detail. It is the difference between a risk that demands immediate action and one that is acceptable background noise.

## The three required elements

A complete risk statement has three parts:

| Element | What it answers | Example |
| --- | --- | --- |
| **Chance** | How probable? | 1 in 1,000 |
| **Consequence** | What happens? | Death |
| **Reference** | Measured over what? | Per flight / per year / per 100,000 miles |

Remove any one and the statement is incomplete:

- "1 in 1,000 per flight" but no consequence → 1 in 1,000 chance of *what*? Turbulence? Delay? Fatality?
- "Death, per flight" but no chance → how often? Every flight? Never?
- "1 in 1,000 chance of death" but no reference → meaningless without knowing the denominator

Gerd Gigerenzer, Director of the Harding Center for Risk Literacy, has spent decades showing that incomplete risk statements cause real harm — particularly in medicine. In *Calculated Risks* (2002), he demonstrates that doctors, patients, and judges routinely misinterpret risk statistics because the reference class is unstated or ambiguous. His rule: always state risks as natural frequencies with an explicit denominator and time frame. Not "there is a 10% chance of side effects" but "out of every 100 patients who take this drug for one year, 10 will experience side effects." The difference is not cosmetic. In controlled studies, comprehension rates roughly double when the reference is made explicit.

## What the reference contains

The reference itself has up to three components:

**Population** — who is exposed? "1 in 1,000 workers" is different from "1 in 1,000 people." Workers on a scaffold have different exposure profiles from the general population. Medical risk literature is precise about this: risk of a drug side effect is per patient taking the drug, not per member of the public.

**Exposure** — what activity or condition? "Per flight" defines a discrete exposure event. "Per mile driven" defines a continuous exposure. "Per year of employment" defines a duration. The choice of exposure denominator changes which activities look risky. Flying looks dangerous per journey and safe per mile. Cycling looks safe per journey and dangerous per mile. Neither comparison is wrong; they answer different questions for different decisions.

**Scope** — over what space and time? This is where the most subtle errors creep in. Narrowing the scope can make the same underlying risk look larger or smaller.

## The scope trap

In 1983, Tversky and Kahneman presented two scenarios to experimental subjects:

> **A.** A massive flood somewhere in North America next year, in which more than 1,000 people drown.
>
> **B.** An earthquake in California sometime next year, causing a flood in which more than 1,000 people drown.

Scenario B was rated as *more probable* than Scenario A — even though B is a strict subset of A. Any flood caused by a California earthquake is, by definition, a flood somewhere in North America. The narrower scenario cannot be more probable than the broader one. Yet people rated it higher because the vivid, specific framing (California, earthquake) made it feel more plausible.

This is the conjunction fallacy, and it demonstrates something important about reference and scope: *narrowing the reference changes the perceived risk, even when the actual risk can only decrease*. California is a subset of the USA. "Next year" is a subset of "eventually." Specifying the scope makes the scenario more vivid and more assessable — but it also, mathematically, reduces the probability space.

Risk matrices fall into this trap routinely. An assessor considering "fall from scaffold on this site this month" is working with a narrower scope than one considering "fall from height across the organisation this year." Both might rate the consequence as "fatal" and the likelihood as "possible" — the same cell on the grid. But the first is a single-site monthly risk and the second is an enterprise annual risk. They are not the same quantity, and they should not appear in the same risk register as comparable numbers without stating the reference.

## How medical risk communication handles this

The medical and public health fields have addressed this more rigorously than safety, largely because the stakes of miscommunication are immediate and personal — a patient deciding whether to accept a treatment.

**David Spiegelhalter** (Cambridge) introduced the *micromort* — a one-in-a-million chance of death — as a standard unit for comparing acute risks. The micromort forces the reference to be explicit: one micromort *per what*? Per surgical procedure. Per skydiving jump. Per 230 miles of motorcycling. Per day of being aged 80. The unit is useless without the exposure, and that is precisely the point — it makes the reference impossible to omit.

Spiegelhalter also introduced the *microlife* (30 minutes of life expectancy) for chronic risks, and insists on a communication rule: never say "10% chance of X." Say "10 out of 100 people *per year*" or "10 out of 100 people *who have this surgery.*" The denominator and the time frame are non-negotiable.

**Gigerenzer** showed that the confusion is not just poor communication — it is structurally embedded in how probabilities are presented. A "30% chance of rain tomorrow" was interpreted by German weather forecasters to mean: it will rain for 30% of the day, it will rain in 30% of the region, or it will rain on 30% of days like tomorrow. Same number, three different meanings, because the reference class was unstated. His solution: natural frequencies with explicit denominators. "Out of 100 days with weather patterns like tomorrow's, it rains on 30 of them." The reference class (100 similar days) is baked into the statement.

**Kaplan and Garrick** (1981), in the foundational paper of risk analysis, defined risk as a set of triplets: scenario, probability, and consequence. They discussed the "relativity of risk" — risk is always relative to the observer and their knowledge. But even the triplet formulation leaves the reference implicit. "Probability" without a denominator is incomplete. Later authors — particularly Aven and the Bayesian school — have argued that probability itself is always conditional on background knowledge and a reference frame, making the reference a fourth element that the triplet assumes but does not state.

## What this means for PIGs

A probability-impact grid typically defines its likelihood axis with words:

| Level | Label | Typical definition |
| --- | --- | --- |
| 5 | Almost certain | Expected to occur in most circumstances |
| 4 | Likely | Will probably occur |
| 3 | Possible | Might occur at some time |
| 2 | Unlikely | Not expected to occur |
| 1 | Rare | May occur only in exceptional circumstances |

None of these specify a reference. "Expected to occur in most circumstances" — over what population, what time frame, what scope of operations? Some organisations add a frequency guide ("likely = once per year"), but this introduces its own problems: "once per year" at what level? Per task, per site, per division, per enterprise? A fall from height that happens once per year across an enterprise of 10,000 workers is a very different risk from a fall that happens once per year at a single 20-person site.

The [distributional approach](percentiles-and-distributions.md) does not solve the reference problem automatically — you still need to specify what the probability is measured over. But it forces the question earlier and more explicitly, because you cannot fit a [metalog distribution](metalog-in-plain-english.md) without specifying what the percentile estimates refer to. "P10 = medical treatment" *for what scenario*? "P90 = fatality" *given what exposure*? The structure of the input — three percentile estimates for a defined scenario — embeds the reference in the question rather than leaving it to the assessor's interpretation.

## The practical test

Before accepting any risk statement — on a PIG, in a report, in a conversation — ask three questions:

1. **Chance of what?** (Is the consequence specified?)
2. **How probable?** (Is the chance quantified, even approximately?)
3. **Measured over what?** (Is the reference — population, exposure, scope — explicit?)

If any element is missing, the statement is incomplete and the risk cannot be compared to any other risk. "High risk" on a grid is not comparable to "high risk" on another grid unless both grids specify the same reference. They almost never do.

## Sources

- Gigerenzer, G. (2002). *Calculated Risks: How to Know When Numbers Deceive You*. Simon & Schuster. (Published in the UK as *Reckoning with Risk*.)
- Gigerenzer, G. (2014). *Risk Savvy: How to Make Good Decisions*. Viking.
- Kaplan, S. and Garrick, B.J. (1981). On The Quantitative Definition of Risk. *Risk Analysis*, 1(1), 11–27.
- Spiegelhalter, D. (2017). Risk and Uncertainty Communication. *Annual Review of Statistics and Its Application*, 4, 31–60.
- Spiegelhalter, D. (2019). *The Art of Statistics: Learning from Data*. Pelican.
- Spiegelhalter, D. (2024). *The Art of Uncertainty: How to Navigate Chance, Ignorance, Risk and Luck*. Allen Lane.
- Tversky, A. and Kahneman, D. (1983). Extensional versus intuitive reasoning: The conjunction fallacy in probability judgment. *Psychological Review*, 90(4), 293–315.
