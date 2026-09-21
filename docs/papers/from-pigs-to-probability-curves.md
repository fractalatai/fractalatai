# From PIGs to Probability Curves

2026-09-21 · Jason Woodruff

Ten years ago I wrote a short paper arguing that probability-impact grids should be scrapped. The tools to replace them existed but were not widely adopted. They now are — and we have built a working system that does what that paper could only advocate.

## The original argument

In 2014 I wrote a three-page paper for Northumbrian Water Group titled *From PIGs to Monte Carlo in 3 Pages*. Its thesis was blunt: probability-impact grids — the 5×5 coloured matrices that sit at the heart of most enterprise risk management processes — should be scrapped.

The argument was not mine alone. Chris Chapman and Stephen Ward, both Professors of Management at the University of Southampton, had written in *How to Manage Project Opportunity and Risk*:

> The probability-impact grid (PIG) — a tool that needs scrapping. PIGs suffer from major inherent limitations (Cox, 2008; Hubbard, 2009), and require a simplistic characterisation of risk and uncertainty that falls well short of a minimum clarity requirement. Their continued use involves a classic 'tail wagging the dog' illustration of much poor 'risk management' practice, and their use should now be scrapped in all contexts.

Douglas Hubbard, in *The Failure of Risk Management*, went further. He devoted an entire chapter to the flaws of PIGs, citing Dr Tony Cox's research at MIT concluding they are "worse than useless" — not merely unhelpful but actively harmful, introducing their own sources of error and making decisions worse than they would have been without any risk assessment at all.

The core criticisms were mathematical and cognitive:

- **Range compression.** Forcing continuous variables into 5 ordinal bins destroys information. A fall from 2 metres and a fall from 20 metres can both land in "high severity." The 10× difference in energy — the difference between a broken wrist and a fatality — disappears.
- **Ambiguous axes.** "Likely" means different things to different people. Without calibration, a room of ten assessors will place the same risk in five different cells.
- **Multiplicative fallacy.** Multiplying ordinal ranks (3 × 4 = 12) is mathematically meaningless. The distance between "unlikely" and "possible" is not the same as between "possible" and "likely," yet the arithmetic treats them as equal.
- **False precision from coarse inputs.** The grid implies a decision framework — red means act, green means accept — but the boundaries between cells are arbitrary and the ranking of risks is unstable under small changes in judgement.
- **No aggregation.** You cannot add two PIGs together to understand portfolio risk. Each assessment is an island.

The paper's conclusion was that the PIG was dead, and that organisations should build competence in probabilistic methods — specifically Monte Carlo simulation — to replace it. The tools existed. The mathematics was not new. What was missing was organisational will.

## What changed

Three developments in the intervening decade have moved the alternative from theoretical to practical.

**The metalog distribution.** In 2016, Tom Keelin published the metalog family of distributions in *Decision Analysis* (see [The metalog in plain English](../sif/metalog-in-plain-english.md)). The metalog is defined by its quantile function — you provide three or more percentile estimates (what is the 10th percentile outcome? the median? the 90th?) and the distribution is fully specified. No choosing between normal, lognormal, beta, or Weibull. No maximum likelihood fitting. Three numbers from a subject matter expert, and the mathematics gives you a continuous probability distribution with a closed-form inverse CDF. This is the missing piece the 2014 paper could not point to: a distribution that meets experts where they already think — in percentiles — and requires no statistical training to parameterise.

**SIPmath 3.0.** Sam Savage, whom the original paper cited, spent the decade building the SIPmath standard through ProbabilityManagement.org (see [Monte Carlo and the Swiss cheese model](../sif/monte-carlo-and-swiss-cheese.md)). A Stochastic Information Packet (SIP) is an array of Monte Carlo trials generated from a metalog. Two SIPs compose by element-wise arithmetic: multiply a severity distribution by (1 − effectiveness) and you get a residual severity distribution. No convolution integrals. No simulation engine. The array *is* the output distribution. The SIPmath 3.0 standard provides a portable JSON format, the HDR random number generator gives reproducible independent streams, and the whole system runs in a spreadsheet, a browser, or on a microcontroller. The corporate sponsors Savage had assembled in 2014 — Lockheed Martin, GE, Chevron — have been joined by Shell, Equinor, and the US intelligence community.

**Energy-based safety.** In 2024, Matthew Hallowell published *Energy-Based Safety* (see [The Energy Wheel](../sif/energy-wheel.md)), operationalising William Haddon's 1973 energy transfer theory into a practical classification framework. The core insight: injury is unwanted energy transfer from source to target. A 5×5 grid asks "how likely?" and "how bad?" — subjective judgements that two assessors will answer differently. The energy framework asks "how many joules?" — a question with a calculable answer. Hallowell's Energy Wheel organises workplace hazards into ten energy types (gravity, motion, mechanical, electrical, pressure, thermal, chemical, radiation, sound, biological), each with empirical severity thresholds derived from decades of OSHA data. The question shifts from opinion to physics.

Together these three developments form a complete replacement for the PIG: the metalog provides the distribution, SIPmath provides the composition algebra, and energy-based safety provides the domain model. None existed in usable form when the original paper was written.

## From grids to distributions

Consider a specific example. A worker falls from a scaffold at 6 metres onto a concrete slab. On a PIG, a safety team might assess this as "likely" severity 4, "possible" probability 3, giving a risk score of 12 — a red cell requiring immediate action. But what does "12" mean? It cannot be compared to another risk scored 12 through a different path (severity 3 × probability 4). It cannot be combined with other risks to understand portfolio exposure. It gives no answer to the question every safety professional actually needs: *could this have killed someone?*

The probabilistic approach starts from physics. A 75 kg worker falling 6 metres onto concrete releases approximately 4,400 joules of gravitational potential energy. From OSHA severe injury data, we know the distribution of outcomes when this energy is transferred to a human body. A [calibrated](../sif/calibrated-estimation.md) subject matter expert — or the historical data directly — provides three [percentile estimates](../sif/percentiles-and-distributions.md):

- **P10** (10th percentile, optimistic outcome): medical treatment
- **P50** (median outcome): serious injury
- **P90** (90th percentile, pessimistic outcome): fatality

These three points define a metalog distribution. The quantile function is closed-form:

$$M(y) = a_1 + a_2 \ln\frac{y}{1-y} + a_3 \left(y - \frac{1}{2}\right) \ln\frac{y}{1-y}$$

where the coefficients are calculated directly from the three estimates — no iteration, no optimisation. From this distribution we read P(SIF) = P(severity ≥ serious injury) directly from the CDF. For the 6-metre fall onto concrete: P(SIF) ≈ 0.78.

This single number — 0.78 — carries more decision-relevant information than any cell on a 5×5 grid. It is continuous, not binned. It is derived from physics and data, not subjective judgement. It can be compared directly to any other P(SIF). And critically, it can be *updated*: add a safety net with its own effectiveness distribution, and the residual P(SIF) drops to 0.02. The grid cannot do this. The grid has no mechanism for asking "what if we add a control?" except to reassess subjectively and hope the assessors shift the cell.

Every incident is a sample from a distribution of possible outcomes. Drop a hundred workers from the same scaffold under the same conditions and they will have a *distribution* of outcomes — some walk away, some are hospitalised, some die. The variables that shift the distribution are source energy, carrier properties, the receiving environment, and body vulnerability. The actual outcome is one draw. SIF potential analysis asks: what does the distribution look like, regardless of what was drawn this time? A near-miss where the worker caught the handrail has the same P(SIF) as a fatality where they fell headfirst onto concrete. The energy available was identical; only the draw differed.

## The SIF classifier

We have built a two-stage classifier that takes a free-text incident or near-miss narrative and produces a [SIF-potential](../sif/what-is-sif-potential.md) classification with structured justification. It replaces the subjective manual review process — where inter-rater agreement sits at roughly 65% (Hallowell & Spencer, 2024) — with a consistent, auditable, physics-grounded assessment.

**Stage 1** answers *what happened*. A fine-tuned small language model classifies the narrative against the WHO International Classification of External Causes of Injury (ICECI) taxonomy, now maintained as ICD-11 Chapter 23. The output is a mechanism code (fall, struck-by, caught-in, electrical contact, etc.) and an object or substance code. Some mechanisms — over-exertion, abrasion — are inherently low-energy and route directly to non-SIF. Everything else passes to Stage 2.

**Stage 2** answers *how much energy was involved*. A second model performs structured extraction against Hallowell's Energy Wheel: it identifies the energy type, extracts source properties from the narrative (heights, speeds, voltages, temperatures, concentrations), detects amplification cues (sharp objects, hard surfaces, head exposure), and estimates severity at three quantiles — P10, P50, P90.

The two stages mirror two distinct questions the PIG conflates into a single cell. "What kind of event was this?" is a classification problem with an established taxonomy. "Could it have killed someone?" is a physics problem requiring estimation of energy magnitude. Separating them gives interpretable intermediate outputs — a safety professional can see the mechanism classification, examine the energy analysis, and understand exactly why the system reached its conclusion. The PIG offers no such chain of reasoning.

From the three severity quantiles, a metalog distribution is fitted and P(SIF) is read from the CDF. The classification is then a policy threshold:

| P(SIF) | Classification | Action |
| --- | --- | --- |
| ≥ 0.50 | SIF | Flag for SIF investigation |
| 0.10–0.50 | Elevated | Review; may warrant investigation |
| < 0.10 | Non-SIF | Standard incident processing |

The threshold is customer-configurable. The model outputs the probability; the organisation decides the response. This separates the scientific question — what is the severity distribution? — from the operational question — what warrants investigation? The PIG merges these and leaves both opaque.

When quantitative information is missing from the narrative — "worker fell from ladder" with no height stated — the system draws on a [Bayesian prior](../sif/bayesian-priors.md): the distribution of ladder fall heights from OSHA data. A contextual clue ("top of extension ladder") narrows the prior substantially. A specific measurement ("6-metre scaffold") collapses it to near-certainty. The system reports the uncertainty: "P(SIF) = 0.45, but height was estimated from prior — specify actual height for more precise classification." Over time, reporters learn what information matters, and data quality improves. No PIG has ever driven this feedback loop.

## The simulator

The classifier screens incidents after they happen. The simulator operates before the work begins — a pre-task planning tool for job hazard analysis that requires no machine learning, no language model, and no narrative text. A safety professional dials in the energy scenario: energy type, height, mass, surface material. The system calculates the severity distribution from calibrated empirical curves and reports P(SIF).

Then the professional adds mitigations. Each mitigation — a safety net, a fall harness, a hard hat — has its own effectiveness distribution, expressed as a bounded metalog on [0, 1]. A safety net does not have a fixed effectiveness of 90%. It has a distribution: P10 = 0.75, P50 = 0.90, P90 = 0.97. Sometimes the net is poorly rigged. Sometimes the fall angle means partial contact. The distribution captures this.

Mitigations compose by element-wise multiplication of Monte Carlo trial arrays — the SIPmath algebra. For each of 10,000 trials:

> residual severity = unmitigated severity × (1 − net effectiveness) × (1 − harness effectiveness)

This is the [quantified Swiss cheese model](../sif/monte-carlo-and-swiss-cheese.md). In trial 4,719 where the net is poorly rigged (effectiveness = 0.3) and the harness is unclipped (effectiveness = 0), the worker faces nearly the full unmitigated severity. In trial 8,002 where both controls perform well, the residual severity is negligible. P(SIF) is simply the fraction of trials where residual severity exceeds the SIF threshold.

This is what the 2014 paper was asking for. Not a 5×5 grid with a red cell and a note saying "install safety net," followed by a reassessment where the cell moves to amber because the assessor feels better about it. Instead: a quantified statement that two barriers reduce P(SIF) from 0.78 to 0.02, with the full distribution visible, the failure modes explicit, and the marginal contribution of each control calculable.

The simulator also handles the dangerous assumption that most risk matrices silently make: independence. If deferred maintenance has degraded the net's rigging *and* the harness inspection regime, these barriers are correlated — when one fails, the other is more likely to fail too. The SIPmath framework models this through a Gaussian copula layer: correlated mitigations share coupled random number streams. In trials where maintenance is poor, both barriers are degraded simultaneously. Simple multiplication of point-estimate reliabilities (0.90 × 0.92 = 0.828) misses this entirely. The simulation does not.

The classifier and the simulator share the same metalog engine and severity model. When the classifier flags an incident as SIF, the user can open it in the simulator, verify the energy parameters the model extracted, adjust where needed, and explore what mitigations would have reduced P(SIF). This closes the loop between post-event screening and pre-task planning — a loop the PIG cannot even represent.

## What this means for enterprise risk management

The 2014 paper ended with a recommendation that Northumbrian Water "critically review its current approach to enterprise risk management." That recommendation was polite and it was ignored, as such recommendations usually are. The PIG is comfortable. It fits on a slide. Everyone has seen one. The alternative looked like hard mathematics.

The alternative no longer looks like hard mathematics. A metalog distribution from three percentile estimates is no harder to elicit than a PIG assessment — arguably easier, because "what is the best realistic outcome, the most likely outcome, and the worst realistic outcome?" is a more natural question than "is this risk likely or possible?" The computation is invisible: the quantile function is three lines of arithmetic, the Monte Carlo composition is element-wise multiplication, and the whole engine compiles to 15 kilobytes of WebAssembly that runs in a browser tab.

But the competence shift is real. Organisations adopting this approach need three capabilities they do not currently exercise:

**[Calibrated estimation.](../sif/calibrated-estimation.md)** The quality of the output depends on the quality of the percentile inputs. Research by Hubbard and others shows that untrained estimators are systematically overconfident — their stated 90% confidence intervals contain the true value only 50% of the time. Calibration training, which takes roughly half a day, corrects this. Organisations that invest in calibrating their subject matter experts get dramatically better inputs to the model. Organisations that do not will still get better outputs than a PIG — the structure imposes discipline even on uncalibrated inputs — but they leave value on the table.

**Thinking in distributions.** A P(SIF) of 0.45 is not "medium risk." It means that in 45% of the ways this scenario could unfold, someone is seriously injured or killed. This is a different cognitive frame from red/amber/green, and it takes practice. The simulator helps: watching a severity distribution shift as you add or remove controls builds intuition about probability that no amount of PIG-based training achieves.

**Valuing information.** Savage's insight from *The Flaw of Averages* — that information has no value unless it can change a decision — becomes operational when you have a model. If P(SIF) = 0.45 and the decision threshold is 0.50, then information that might move the estimate across the threshold is valuable; information that cannot is not. The system can calculate the expected value of perfect information for any uncertain input: "if you measured the actual fall height, the expected reduction in decision uncertainty is worth £X." This is decision theory made practical. The PIG has no mechanism for it because the PIG has no model underneath.

Chapman and Ward wrote that "organisations that successfully adopt an uncertainty management perspective will have a significant advantage over those that do not." A decade later, the advantage is no longer theoretical. It is a working system that classifies SIF potential from free text, quantifies severity distributions from energy physics, models mitigation effectiveness with real uncertainty, and runs on a laptop without sending data to the cloud.

## Personal

I wrote the original paper having spent over twenty years in safety and risk management without once being convinced that a probability-impact grid worked. In personal safety we are not really dealing with much uncertainty — we know what hurts people. The physics of falling, of electrical contact, of crushing, of thermal exposure are well understood. The question was always one of magnitude, not of category. A red cell never told anyone whether a fall was survivable.

Since then I have built Monte Carlo simulations with project managers at Siemens Service Renewables, where project plans were almost always behind schedule and over budget — and where the single-point estimates on Gantt charts were precisely the "flaw of averages" Savage described. I have watched organisations spend more money gathering information that cannot change a decision than it would cost to gather the information that could.

The SIF classifier and simulator represent the convergence of threads I have been following for thirty years: Haddon's energy transfer theory from 1973, Savage's probability management from 2009, Keelin's metalog from 2016, Hallowell's energy-based safety from 2024, and the edge AI capability that makes it all run on a corporate laptop without an internet connection. The original paper was a signpost. This is the destination — or at least a waypoint. The PIG is still dead. But now we have something to bury it with.

## Sources

- Baker, S.P. et al. (1974). The Injury Severity Score. *Journal of Trauma*, 14(3), 187–196.
- Campbell Institute & DEKRA (2017). *Perspectives on SIF Prevention*.
- Chapman, C. and Ward, S. (2011). *How to Manage Project Opportunity and Risk*, 3rd ed. Wiley.
- Cox, L.A. (2008). What's wrong with risk matrices? *Risk Analysis*, 28(2), 497–512.
- Haddon, W. (1973). Energy Damage and the Ten Countermeasure Strategies. *Human Factors*, 15(4), 355–366.
- Hallowell, M.R. (2024). *Energy-Based Safety*. CRC Press/Routledge.
- Hallowell, M.R. et al. (2017). Energy-based safety risk assessment. *Construction Management and Economics*, 35(1-2).
- Hallowell, M.R. and Spencer, G. (2024). Energy-Based Safety. *Professional Safety* (ASSP).
- Hubbard, D.W. (2009). *The Failure of Risk Management*. Wiley.
- Hubbard, D.W. (2010). *How to Measure Anything*, 2nd ed. Wiley.
- Keelin, T.W. (2016). The Metalog Distributions. *Decision Analysis*, 13(4), 243–277.
- Savage, S.L. (2009). *The Flaw of Averages*. Wiley.
- WHO (2004). International Classification of External Causes of Injury (ICECI), v1.2.
- WHO (2019). ICD-11 Chapter 23: External Causes of Morbidity or Mortality.
