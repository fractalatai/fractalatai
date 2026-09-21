# Consequence is not fixed: how likelihood leaks into consequence on a risk matrix

## The orthodox position

Many safety professionals are taught — and many risk assessment procedures require — that when calculating residual risk after controls are applied, consequence stays the same and only likelihood changes. The reasoning sounds solid: a fall from 6 metres onto concrete is catastrophic whether or not a guard rail is present. The guard rail reduces the likelihood of falling, not the severity of the fall if it happens. As one widely-used training resource puts it: "severity remains relatively static... unless there is a significant change made to remove the hazard or substitute the methods used."

This principle is embedded in risk assessment templates across industries. Assessors rate inherent risk (before controls), then rate residual risk by moving leftward along the likelihood axis while holding the consequence column fixed. It appears in safety management systems from Australia to the UK to the US. It is the default approach.

It is also wrong — or more precisely, it is an artefact of the tool rather than a property of reality.

## Where the logic breaks

The argument rests on a specific framing: *if the event happens exactly as described, the consequence is the same.* A 6-metre fall onto concrete has the same consequence with or without a guard rail — because the guard rail's job is to prevent the fall, not to soften it.

But what about a safety net?

A safety net does not prevent the fall. The worker still falls. The net catches them after 1–2 metres instead of 6. The energy absorbed by the body drops from ~4,400 joules to ~750 joules. The most likely outcome shifts from serious injury to bruising. The consequence has changed — dramatically.

What about a fall harness? The worker falls, the harness arrests the fall, and they swing. The deceleration force is distributed across the body by the harness webbing. The energy transferred to the person is a fraction of an uncontrolled ground impact. Again, the consequence has changed.

What about a hard hat? A falling spanner strikes a worker's head. Without the hat: skull fracture, possible fatality. With the hat: concussion, medical treatment. The event happened — the object struck the person. But the consequence is different because the hat absorbed and distributed the impact energy.

These are not edge cases. They are some of the most common safety controls in existence:

| Control | Does it prevent the event? | Does it change the consequence? |
| --- | --- | --- |
| Guard rail | Yes (prevents fall) | No (if you fall, you fall the full height) |
| Safety net | No (you still fall) | **Yes** (arrests fall at shorter distance) |
| Fall harness | No (you still fall) | **Yes** (distributes deceleration force) |
| Hard hat | No (object still strikes you) | **Yes** (absorbs and distributes impact) |
| Arc flash PPE | No (arc flash still occurs) | **Yes** (reduces burn severity) |
| Emergency shutdown | No (process still fails) | **Yes** (limits energy released) |
| Fire suppression | No (fire still ignites) | **Yes** (extinguishes before flashover) |
| Machine guarding | Yes (prevents contact) | No (if contact occurs, severity unchanged) |
| LOTO | Yes (prevents energisation) | No (if energised, severity unchanged) |
| Spill bunding | No (spill still occurs) | **Yes** (contains volume, limits spread) |

Roughly half the controls in a typical risk register are consequence-reducing controls. The orthodox position that "only likelihood changes" is true for *prevention* controls (left side of the bowtie) and flatly false for *mitigation* controls (right side of the bowtie).

## Why the PIG forces the error

The risk matrix has two axes: likelihood and consequence. Each axis has 5 ordinal levels. The assessor picks one cell.

The consequence axis asks: "if this event happens, how bad is it?" But the word "it" is doing enormous work. Does "it" mean:

- The hazard is present (a 6-metre scaffold exists)?
- The energy is released (the worker falls)?
- The energy reaches a person (the worker hits the ground)?
- The energy causes maximum harm (the worker lands headfirst on concrete)?

The orthodox position implicitly picks the last interpretation: the worst-case energy transfer, with no mitigation and no lucky angles. The consequence is fatality because *if everything goes wrong*, a 6-metre fall kills. This is a defensible choice — it is conservative, and in SIF analysis we deliberately set chance P = 1 (see [What is SIF potential?](what-is-sif-potential.md)).

But then the PIG has no way to represent what the safety net does. The net does not change the "worst case if everything goes wrong" (the net could fail). It changes the *distribution* of outcomes — making serious injury far less likely and fatality rare rather than common. The PIG has no distribution. It has one cell. So the only axis left to absorb the net's benefit is likelihood.

This is what "likelihood leaking into consequence" means. The assessor thinks: "with the net in place, a fatality is unlikely." They move the likelihood from "possible" to "unlikely." The consequence stays at "catastrophic." The risk score drops from 12 to 8. Somewhere in that likelihood shift is an unacknowledged truth: the net changed the consequence distribution, not the probability of falling. The worker is just as likely to fall — the net does nothing about that. What changed is what happens when they do.

## The bowtie makes this visible

The bowtie model separates the two sides cleanly:

```
Threats → [Prevention barriers] → TOP EVENT → [Mitigation barriers] → Consequences
              (reduce likelihood)                  (reduce consequence)
```

- **Left side** (prevention): guard rails, LOTO, permits, training, interlocks. These reduce the probability of the top event occurring. On a PIG, these legitimately shift the likelihood axis.

- **Right side** (mitigation): safety nets, PPE, harnesses, fire suppression, emergency response, spill containment. These reduce the severity of consequences *given the event occurs*. On a PIG, these should shift the consequence axis — but the orthodoxy forbids it.

The PIG has one "likelihood" axis carrying the load of both sides. A guard rail (prevention) and a safety net (mitigation) both manifest as likelihood reductions, even though they do completely different things. The assessor cannot distinguish between "less likely to fall" and "less severe when you do fall" within the matrix framework.

## A classification that makes the split explicit

The bowtie gives a two-way split: prevention (left) and mitigation (right). A more rigorous control classification uses three dimensions — PROPERTY, METHOD, and EVENT — borrowed from object-oriented programming, where an object has properties (what it is), methods (what it does), and responds to events (when it operates).

**METHOD — what the control does to the risk:**

| Method | Type | What it reduces | Can it reach zero? |
| --- | --- | --- | --- |
| **Protection** | Mitigation | Severity of outcome given the event occurs | No — cannot eliminate all energy transfer |
| **Exposure duration** | Prevention | Time a person is in the hazard zone | Approaches zero — but see below |
| **Chance per unit exposure** | Prevention | Probability of the event per unit time exposed | No — residual probability while exposed |

This is the critical decomposition. The PIG's "likelihood" axis is actually three different things compressed into one: *are they there?* (exposure duration), *will it happen while they're there?* (chance per unit exposure), and *how bad will it be?* (protection/severity). Only the first two belong on a likelihood axis. The third — protection — is consequence reduction, and the PIG has nowhere to put it.

Of the three methods, only exposure duration can approach zero — and this is the link to the traditional hierarchy of controls. "Elimination," the top of the hierarchy, is not a separate category of control. It is exposure duration driven towards zero. And it operates in two directions: remove the hazard from the target (design out the energy source), or remove the target from the hazard (take the person out of the zone). Prefabrication at ground level does the first. Automation does the second. Both drive exposure duration towards zero.

But towards zero is not zero. On 21 December 1988, the residents of Lockerbie had reduced their personal exposure to aviation risk to the minimum available — they were not flying. Pan Am Flight 103 fell on them. A person who chooses never to enter a confined space still lives in a world where confined-space atmospheres can migrate through drains, ducts, and basements. Exposure duration is asymptotic: it can be driven very low, but the energy sources in the environment around us mean that true zero exposure is a theoretical limit, not an achievable state.

This asymptotic property is important because it means all three methods behave the same way: they reduce risk but cannot eliminate it. Protection cannot absorb all energy. Chance per unit exposure cannot reach zero while someone is present. And exposure duration cannot reach absolute zero because the world contains energy that does not respect the boundaries we draw around hazard zones. The honest answer for all three methods is "approaches but never reaches zero" — which is precisely what a [probability distribution](percentiles-and-distributions.md) expresses and a risk matrix cannot.

**The link to operational efficiency.** Recognising exposure duration as a distinct control method brings an unexpected set of disciplines into the safety conversation. Anything that reduces the time a person spends in a hazard zone is, by definition, a safety control — even if it was never designed as one:

- **Task planning**: a well-sequenced scaffold erection that takes 4 hours instead of 6 has reduced exposure duration by a third.
- **Prefabrication and modular construction**: assembling at ground level and lifting into place eliminates hours of work at height.
- **Automation and robotics**: a drone inspecting a confined space removes the person entirely for that task.
- **Logistics and traffic management**: separating pedestrian and vehicle routes in time (not just space) reduces the duration of shared exposure.
- **Lean operations**: eliminating waste and waiting time on a hazardous task is simultaneously an efficiency gain and a safety gain.
- **Shift design**: shorter rotations in high-noise or high-heat environments reduce cumulative energy exposure.

These are not traditionally thought of as safety controls. They appear in operations, planning, and engineering departments, not in the safety management system. But the METHOD axis reveals them for what they are: exposure duration controls operating through design, engineering, or management. An organisation that improves the efficiency of a hazardous task has — whether it knows it or not — reduced the risk by reducing the time a person is exposed to the energy source. The safety function and the operations function are working on the same variable from different directions.

This reframing also explains why "elimination" is so hard in practice. The hierarchy of controls presents it as the ideal — eliminate the hazard — but gives little guidance on how. The exposure duration method makes the mechanism concrete: to approach elimination, reduce the time the target and the hazard coexist. Sometimes that means removing the hazard (lower voltage, less toxic substitute). Sometimes it means removing the target (automate, prefabricate, redesign the process). And sometimes it means making the coexistence briefer (faster task, better planning, fewer people for less time). All three are exposure duration controls; all three approach elimination without ever quite reaching it.

**PROPERTY — how the control is implemented:**

| Property | Meaning | Example (fall from height) |
| --- | --- | --- |
| **Design** | Inherent safety — engineered out at source | Prefabricate at ground level (eliminates the need to work at height) |
| **Engineer** | Physical barrier or system | Safety net, guardrail, harness anchorage |
| **Management** | Procedure, training, administrative | Permit to work, toolbox talk, exclusion zone signage |

The PROPERTY axis maps roughly to the hierarchy of controls (elimination/substitution → engineering → administrative/PPE) but crosses it with METHOD to expose combinations the hierarchy alone does not distinguish.

**The 3 × 3 METHOD × PROPERTY matrix:**

| | Design | Engineer | Management |
| --- | --- | --- | --- |
| **Protection** (mitigation) | Lower-voltage system design (reduces energy at source) | Safety net, hard hat, arc flash PPE | Emergency response procedure, first aid training |
| **Exposure duration** (prevention) | Prefabricate at ground level; automate the task | Interlocked access gates; timed permit systems | Permit-to-work duration limits; shift rotation |
| **Chance per unit exposure** (prevention) | Inherently stable structure design | Guardrails, LOTO, machine guarding | Training, toolbox talks, supervision |

Every cell is a distinct type of control doing a distinct thing. The PIG takes this entire 3×3 matrix and folds it into a single shift along the likelihood axis. Nine meaningfully different control strategies become one indistinguishable movement on the grid.

**EVENT — when the control operates:**

The third dimension classifies operating conditions:

| Event | Description | Control behaviour |
| --- | --- | --- |
| **Normal** | Routine operations | Most controls function as designed |
| **Abnormal** | Non-routine: maintenance, start-up, shutdown, changeover | Many engineered controls are bypassed or removed (guards off during maintenance, LOTO procedures activated) |
| **Emergency** | Loss of control | Emergency controls activate (shutdown systems, fire suppression, evacuation); routine controls may be overwhelmed |

A machine guard (Engineer × Chance) works during normal operations. During abnormal operations (maintenance), it is often removed — the very condition where the hazard is most actively present. The PIG assesses the control once, in a static snapshot. It does not ask: "does this control work during maintenance?" The EVENT dimension forces that question.

This matters for SIF because abnormal and emergency events are disproportionately represented in SIF data. The Campbell Institute's research shows that SIF events cluster during non-routine work — exactly when routine controls are suspended. A control portfolio that looks strong under normal conditions may be hollow during the conditions that generate fatalities.

The EVENT dimension also connects to the [correlated failure problem](monte-carlo-and-swiss-cheese.md): during an emergency, multiple controls fail simultaneously because they share a common cause (the emergency itself). A fire overwhelms both the engineering control (fire-rated enclosure) and the management control (evacuation procedure) at the same time. The 3 × 3 × 3 classification makes it possible to ask: "which of our controls survive an abnormal or emergency event?" — a question the PIG never poses.

## The distributional resolution

With a [probability distribution](percentiles-and-distributions.md) instead of a single cell, this confusion dissolves.

**Unmitigated scenario** (6-metre fall, no controls):
- P10: medical treatment
- P50: serious injury
- P90: fatality
- P(SIF) = 0.78

**With safety net** (arrests fall at ~1.5 m):
- P10: no injury
- P50: first aid
- P90: medical treatment
- P(SIF) = 0.03

The consequence distribution has shifted leftward. The likelihood of falling has not changed — the net does not prevent falls. But P(SIF) dropped from 0.78 to 0.03 because the *severity distribution* changed. The net absorbed most of the energy.

**With guard rail** (prevents fall):
- The severity distribution is unchanged (if you fall, you fall the full 6 m)
- But the probability of falling is much lower
- P(SIF) incorporates both: even though the consequence distribution is severe, the event is rare

The distribution framework separates what the PIG conflates. Prevention controls change how often the event occurs. Mitigation controls change what happens when it does. Both reduce P(SIF), but through different mechanisms — and the distinction matters for deciding which controls to invest in.

## Why this matters in practice

An organisation that holds consequence fixed and adjusts only likelihood will systematically:

1. **Undervalue mitigation controls.** If consequence cannot change, then a safety net and a guard rail look the same on the matrix — both shift likelihood by one column. But they serve completely different functions: the guard rail prevents the event; the net survives the event. An organisation that loses the guard rail and keeps the net is in a very different position than one that loses the net and keeps the guard rail. The matrix cannot express this.

2. **Misallocate resources.** If the only way to reduce risk score is to reduce likelihood, the organisation will over-invest in prevention and under-invest in mitigation. This is particularly dangerous for SIF events, where the empirical evidence (Campbell Institute, DEKRA) shows that SIF precursors are different from minor-injury precursors. Reducing the frequency of minor trips does not reduce the severity of a fall from height.

3. **Create a false floor.** Once likelihood is reduced to "rare," the risk score stops falling even though additional mitigation controls would further reduce the actual severity distribution. The assessor has no cell left to move to. The residual risk appears acceptable because the matrix ran out of room, not because the risk is actually controlled.

4. **Miss the correlated failure case.** When deferred maintenance degrades both the guard rail and the net simultaneously, the likelihood of falling *increases* (guard rail degraded) at the same time that the severity given a fall *increases* (net degraded). The PIG cannot represent this interaction because it put both controls on the same axis.

## The one thing the orthodox position gets right

There is a kernel of truth in the orthodox position: *you should not assume controls will work.* A SIF analysis deliberately excludes controls to reveal the underlying energy hazard (see [What is SIF potential?](what-is-sif-potential.md)). The unmitigated consequence — what happens if everything fails — is a critical reference point.

The error is not in assessing the unmitigated consequence. The error is in having no mechanism to then show how mitigation controls shift the distribution. The PIG forces a binary: either the consequence is fixed at the unmitigated worst case, or it is arbitrarily moved by committee feel. The distribution provides a principled middle ground: the unmitigated distribution is calculated from physics, and each control's effect on the distribution is calculated from its [effectiveness data](monte-carlo-and-swiss-cheese.md). Both the raw exposure and the residual risk are quantified, traceable, and auditable.

The PIG is not wrong to start at the worst case. It is wrong to stay there.

## Sources

- Cox, L.A. (2008). What's wrong with risk matrices? *Risk Analysis*, 28(2), 497–512.
- Hallowell, M.R. (2024). *Energy-Based Safety*. CRC Press/Routledge.
- Hubbard, D.W. (2009). *The Failure of Risk Management*. Wiley.
- [Risk Edge Group — You Can Reduce Consequence, Right?](https://www.riskedge.com.au/you-can-reduce-consequence-right/)
- [Safeti — How to Assess the Risk](https://safeti.com/how-to-assess-the-risk-health-and-safety-masterclass/)
- [Resilience Explorer — Risk Matrices Can Mislead](https://resilience-explorer.com/articles/risk-matrices-can-mislead-more-than-they-help)
