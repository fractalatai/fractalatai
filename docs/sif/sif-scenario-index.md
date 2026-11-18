# PSIF scenario index: the risk-assessment side

*Version 0.1, 2026-11-18. Draft for discussion.*

## Summary

Triagers and the panel code PSIF **from reports**: an event happened, someone wrote it down, and we judge its potential. The same rules can be run **from the other direction**. Take an activity and its energy sources, apply the coding rules to every credible change on either side (excursion or incursion), and record each one as a **scenario**: a PSIF event defined before it happens. No actual event is needed. That is the basis of risk assessment as a proactive method.

The result is a searchable **PSIF scenario index**. Each row is a credible event with its potential, its people at risk and the direct controls that stop it. Coded reports point back to the index, so every scenario gathers a history: how often it happened, how close it came and which control held or failed.

Range operations make a good pilot because the MoD has already drawn most of the envelopes in numbers. Weapon and range danger areas, burst safety distances, backsplash distances and backblast areas are all specified, and the controls are written into Range Standing Orders (DSA 03.OME Part 3, formerly JSP 403). A first pass over the range code gives **36 scenarios**:

- 26 are PSIF if they happen;
- 4 are Elevated;
- 6 are SIF exposures, which are conditions and not events.

Four findings matter before a pilot:

1. **Most range PSIF events have a wide closest approach.** A round over the stop butt or a walker inside the danger area during firing is PSIF even when nobody is near. The MoD already asks for incursions to be recorded (DCOP 313 para 22), so the data partly exists.
2. **Several low-drama reports are PSIF under our rules.** A weapon found loaded after the unload drill, a live round in blank ammunition, a muzzle swept across a person and a walk into a closed impact area out of hours are all likely to be logged as administrative or minor today.
3. **The band table has no ballistic band.** A stray rifle round would fall into `struck/swinging_projectile` (P = 0.439, Elevated), which under-rates it. A `ballistic` mechanism is needed before any range pilot (section 5).
4. **The MoD's vocabulary is already two-sided.** It uses "incursion" for people entering the danger area. Its accident rule covers "un-intended functioning" of weapons and munitions, which is an excursion. Its weapon danger area is our envelope. One term clashes: the MoD's "line of fire" is the barrel-to-target line, not our path of live energy (section 6).

## 1. Why index scenarios

| | Report-led coding (today) | Scenario index (proposed) |
|---|---|---|
| Starting point | An event that happened and was reported | An activity, its energy sources and their envelopes |
| Question | What was the potential of this event? | What events could this activity produce, and which are PSIF? |
| Who does it | Triager, AI screen, panel | Panel and activity owners, once per activity, reviewed periodically |
| Covers | What gets reported | What could happen, reported or not |
| Output | One determination per report | A catalogue of credible PSIF events per activity, with their controls |
| Gaps it shows | Under-reporting is invisible | Scenarios with no reports are visible: either rare or not reported |

The two directions share one rulebook ([sif-definition.md](sif-definition.md) §7). A scenario is coded exactly as a report would be: by energy source, event type (excursion or incursion; where, when or how much), mechanism and band, and P(serious). Two fields differ because no real event exists:

- **Closest approach** is not set on the scenario. It belongs to each real event linked to it.
- **Control state** is not set either. The scenario lists the direct controls that should stop it; each linked event records whether they did.

The index does not replace coding. A report is still coded on its own facts. The scenario supplies a default band and the list of controls to check, and the facts stated in the report override it: an air rifle is not a service rifle.

**Related practice.** The nearest published idea is the ICMM *material unwanted event* (MUE) ([sif-definition.md](sif-definition.md) §5.7): an event, defined in advance, that critical controls must prevent. A SIF-class scenario is an MUE stated in the two-sided event terms, with its potential read from the band table rather than judged per site. The direct controls listed against SIF-class scenarios are the candidate critical controls.

## 2. Schema

The index has six tables. The **scenario** is the core; the others describe what it is built from and link it to real reports.

```
activity ─┐
          ├─< scenario >─< scenario_control >─ control
source  ──┘       │
                  └─< report_scenario >─ (coded report)
```

### 2.1 `scenario`

| Field | Type | Content | Example |
|---|---|---|---|
| `scenario_id` | text, key | `<domain>-<side>-<dimension>-<nn>`. Side: `EX` excursion, `IN` incursion, `SX` SIF exposure. Dimension: `WR` where, `WN` when, `HM` how much | `RNG-EX-WN-02` |
| `title` | text | The change, in plain words | Weapon found loaded after the unload drill |
| `activity_ids` | list | Activities where it can occur | `rng.nda_barrack`, `rng.lftt`, `rng.wuf` |
| `source_id` | key | Energy source | `src.sa_centrefire_rifle` |
| `energy_type` | enum | From §2.1 of the definitions | motion |
| `energy_state` | enum | State in normal operation: `live`, `at-rest`, `zero-energy` | live |
| `envelope` | text + ref | The designed bound that is crossed, with the clause that sets it | Weapon declared clear (zero-energy) by unload and proving drill |
| `danger_zone` | text + ref | The area kept clear of people, if any | Range danger area (RDA) |
| `side` | enum | `excursion`, `incursion`, `both`, `exposure` | excursion |
| `dimension` | enum | `where`, `when`, `how_much` | when |
| `description` | text | What changes, and why that shrinks the separation to people | The weapon was declared zero-energy but a round was chambered: energy is live when it should not be |
| `people_at_risk` | list | Roles (§2.4) | firer, adjacent firers, armourer |
| `receiving_environment` | text | What the energy would strike; body region at risk | Torso or head at close range |
| `mechanism` / `band` | enum | From Annexes A and B | `ballistic/centrefire_rifle` |
| `p_serious` | number | From the band | 0.72 (provisional) |
| `class` | enum | SIF, Elevated, Not SIF | SIF |
| `band_status` | enum | `calibrated`, `provisional` (band missing or borrowed) | provisional |
| `if_event` | enum | The determination a matching report gets with no injury: PSIF, Elevated, Not SIF, or SIF exposure | PSIF |
| `triggers` | list | Conditions that make it more likely (chance only; no effect on potential) | Stoppage cleared in a hurry; drill done out of sequence |
| `related_exposure` | key | The SIF exposure (`SX-`) this event comes from | `RNG-SX-01` |
| `report_phrases` | list | Words people use in reports, for search and AI matching | "not cleared", "round in chamber", "unload drill" |
| `source_refs` | list | Clauses the scenario is built from | DSA 03.OME Pt 3 Vol 1 Pt 2 para 9 |
| `notes` | text | Boundary cases, coding notes | Proved clear and inspected: zero-energy, no event |
| `status` | enum | `draft`, `panel_reviewed`, `retired` | draft |
| `version`, `reviewed_on` | text, date | Change control | 0.1, — |

### 2.2 `activity` and `source`

| Table | Fields | Example |
|---|---|---|
| `activity` | `activity_id`, `name`, `domain`, `range_type` (MoD category), `practices`, `source_refs` | `rng.grenade`, Grenade range, ranges, Vol 2 Ch 18 |
| `source` | `source_id`, `name`, `energy_type`, `magnitude` (value + unit: muzzle energy J, NEQ kg, speed km/h), `default_band`, `envelope_type` (WDA template, BSD, backsplash, backblast area, ADH), `source_refs` | `src.hand_grenade`, HE hand grenade, chemical/pressure + fragments, BSD from Pamphlet 21 |

### 2.3 `control` and `scenario_control`

| Field | Content |
|---|---|
| `control_id`, `name` | `ctl.clear_range_procedure`, Clear Range Procedure |
| `type` | active, passive, procedural (definitions §2.4) |
| `direct` | yes or no. Training, rules, signs and generic PPE are not direct (EEI) |
| `acts_on` | source (stops the excursion) or path (keeps people out: stops the incursion) |
| `source_refs` | The clause that requires it: Vol 2 para 2-13 |
| `scenario_control.role` | `prevents` (stops the event) or `mitigates` (limits harm once it starts) |

### 2.4 Roles (`people_at_risk`)

Ranges have fixed roles, which keeps searches clean: firer, adjacent firer, range conducting officer (RCO), safety supervisor or safety staff, butt marker, range warden or sentry, EOD operator or clearance team, spectator, authorised public (permissive access), unauthorised public, aircrew, range maintenance staff.

### 2.5 `report_scenario` (the link back)

| Field | Content |
|---|---|
| `report_id` | The coded report |
| `scenario_id` | The matched scenario; `none` if no scenario fits (a candidate for a new row) |
| `matched_by` | triager, AI model, panel |
| `match_confidence` | 50–100% (the panel's confidence scale) |
| `closest_approach` | wide, close, contact |
| `control_state` | effective, failed or absent, unverified, naming the control |
| `determination` | From definitions §7.3 |

With this link the index answers the questions a risk assessment cannot answer alone:

- How often did each scenario happen?
- What share came close or made contact?
- Which control failed most?
- Which SIF-class scenarios have never been reported?

### 2.6 Searches it supports

| Question | Query on |
|---|---|
| What PSIF events can a butt marker suffer? | `people_at_risk` |
| What does the Clear Range Procedure protect against, and what is left if it fails? | `scenario_control` |
| Which range scenarios are excursions in time? | `side`, `dimension` |
| A report says "round went over the butts": which scenario, which controls to check? | `report_phrases`, then `scenario_control` |
| Which scenarios borrow a band? | `band_status = provisional` |
| Which SIF-class scenarios have no linked reports in 12 months? | `report_scenario` |

### 2.7 Example record

```yaml
scenario_id: RNG-IN-WR-04
title: Person enters a closed impact area
activity_ids: [rng.mortar, rng.grenade, rng.at_weapon, rng.lftt]
source_id: src.blind_fuzed_ordnance
energy_type: chemical          # explosive fill, held by a fuze
energy_state: live             # the fuze can function on disturbance
danger_zone: Closed impact area, fenced, "No Entry to Impact Area" (Vol 2 para 2-106)
side: incursion
dimension: where
description: Entry past the demarcation fence brings a person within reach of blinds at any time, firing or not
people_at_risk: [range maintenance staff, firer, unauthorised public]
mechanism: explosion
band: fragmenting_munition      # proposed (section 5)
class: SIF
band_status: provisional
if_event: PSIF
related_exposure: RNG-SX-04    # EOD clearance is planned presence, so exposure
report_phrases: ["impact area", "past the fence", "UXO", "blind", "recovered a target"]
source_refs: ["DSA 03.OME Pt 3 Vol 2 para 2-25", "para 2-106"]
status: draft
```

## 3. How it is built

The build applies the coding steps forwards, once per activity:

1. **List the energy sources** for the activity: each weapon and ammunition nature, explosive store, vehicle, and anything else in Annex A.
2. **Write each source's envelope** in the three dimensions, taken from the code where it exists:
   - where: the WDA template, burst safety distance, backsplash distance, backblast area, air danger height;
   - when: the firing window, waiting times after a misfire, the state the weapon was declared to be in;
   - how much: the authorised natures, muzzle energy limits and net explosive quantity (NEQ).
3. **Write the people's envelope**: roles, where each is planned to be, when, and how many.
4. **Run each cell of the two-envelope table** ([sif-definition.md](sif-definition.md) §3.2). For each source, ask what excursion and what incursion is credible in where, when and how much. Keep the credible ones as scenarios.
5. **Code each scenario**: mechanism, band, receiving environment, class. Planned presence within reach becomes an `SX-` exposure, not an event.
6. **List the direct controls** from the code and Range Standing Orders against each scenario, marking which prevent and which mitigate.
7. **Panel review**: challenge credibility, the bands and the control lists. Set `status: panel_reviewed`.

Two rules from the definitions carry over unchanged:

- **Separation to zero.** The potential assumes the energy reached a person. A round that leaves the danger area over empty moorland has the same potential as one that lands by a walker.
- **The number of people does not change potential.** A spectator stand inside a safety distance is one scenario per person; it multiplies exposure and the number who could be harmed in one event.

## 4. Pilot: range operations

### 4.1 Why ranges

- **The envelopes are drawn in numbers.** The MoD defines the weapon danger area as "the space into which the weapons ammunition or it's fragments may travel, impact or function given normal firing conditions" (Vol 2 para 2-48). That is our envelope for an intentional release ([sif-definition.md](sif-definition.md) §2.1). Templates, ricochet angles (30° off soft and 45° off hard targets for high-velocity ammunition, para 2-41), backsplash distances (Table 2-2) and burst safety distances are tabulated.
- **The code names its own residual.** The WDA "excludes low probability human and system errors" (para 2-48). Those errors are exactly the excursions the index lists.
- **The danger zone is defined and controlled.** The range danger area is "the space within a range in which there may be a risk to personnel, equipment, or property from firing authorised weapons within specific arcs" (para 2-38). The Clear Range Procedure keeps it "clear of unauthorised persons before firing commences and that it remains clear throughout the time firing is in progress" (para 2-13).
- **Incursions are already a concept.** The code requires that "firing to be stopped as soon as any incursion is imminent" (Vol 1 Pt 2 para 99), and that "all incursions and prevented incursions should be recorded" (DCOP 313 para 22).
- **Roles and activities are finite.** Range types are catalogued (Vol 2 Ch 1–27), and roles are set in Range Standing Orders.

### 4.2 Sources used

All five are DSA 03.OME Part 3, Defence Code of Practice (DCOP) and guidance notes for ranges, published by the Defence OME Safety Regulator (DOSR) on GOV.UK. JSP 403 is withdrawn; its GOV.UK collection says the content is "out of date" and "should only be used as guidance until replaced".

| Short name | Document | Version |
|---|---|---|
| Vol 1 Pt 1 | Policy, regulation, legislation, responsibilities | Jun 2020 |
| Vol 1 Pt 2 | Range management (responsibilities, use, public access, inspection) | Oct 2019 |
| Vol 2 | Range design, construction and danger areas, by range type | v2.2 (GOV.UK, Sep 2026) |
| Vol 4 | Demolitions, EOD and battle simulation | Apr 2019 |
| DCOP 313 | Fieldcraft Training Areas | May 2026 |

Quotations are checked against the local copies in `docs/sif/lit/ranges/` (gitignored). Paragraph numbers are as printed. Ammunition natures and distances here are illustrative; the pilot would take them from the current tables.

### 4.3 What surfaces: PSIF scenarios

All of these are **PSIF if they happen without injury**, determined on separation to zero. Bands marked † use the proposed `ballistic` mechanism or a borrowed band (section 5) and are provisional.

**Excursions: the energy changes**

| ID | Scenario | Dimension | People at risk | Band | Direct controls (prevent) |
|---|---|---|---|---|---|
| RNG-EX-WR-01 | Round leaves the RDA: over the stop butt, out of arcs, wrong elevation | where | Unauthorised and authorised public, other range users | `ballistic/centrefire_rifle` † | Stop butt, baffles, arc limiters (passive); RDA cleared (procedural) |
| RNG-EX-WR-02 | Ricochet leaves the RDA (hard ground, stones on the range floor, a struck steel target) | where | As above | `ballistic/centrefire_rifle` † | Soft range floor, ricochet-safe targetry, maintenance (passive) |
| RNG-EX-WR-03 | Unintentional discharge on the firing point, at the unloading facility, in a vehicle or in accommodation | where | Firer, adjacent firers, anyone in the muzzle path | `ballistic/centrefire_rifle` or `handgun` † | Weapon unloading facility bullet trap (passive); safety catch (passive, can be undone) |
| RNG-EX-WR-04 | Muzzle of a loaded weapon sweeps a person | where | Adjacent firers, safety staff, markers | `ballistic/*` † | None direct: muzzle discipline is a rule |
| RNG-EX-WR-05 | Grenade dropped in the bay or thrown short | where | Thrower, safety supervisor | `explosion/fragmenting_munition` † | Bay design: sump and blast wall (passive); supervisor drills (procedural) |
| RNG-EX-WR-06 | Indirect-fire round lands outside the impact area (short round, wrong charge or bearing) | where | Anyone in the RDA margins, observation posts | `explosion/fragmenting_munition` † | Safety checks of data and laying (procedural); impact area set back from the RDA edge by the BSD (passive, by design) |
| RNG-EX-WR-07 | Fragments or debris from a demolition beyond the calculated safety distance | where | Firing party, sentries | `explosion/significant_industrial` or higher | Safety distance tables (Vol 4 Annex C), protected shelter (passive) |
| RNG-EX-WR-08 | Range vehicle, moving target carrier or armoured vehicle leaves its route or strikes a person | where | Dismounts, range staff | `transport/yard_speed` | Route segregation (passive); vehicle safety staff (procedural) |
| RNG-EX-WR-09 | Range fire started by tracer or pyrotechnics spreads to people | where | Troops in the area, public at the boundary | `fire/structural_fire` † (wildland fire has no band) | Fire breaks, firing restrictions in dry weather (procedural) |
| RNG-EX-WN-01 | Firing starts or continues while the RDA is not clear: before "range clear", with markers exposed, or after "stop" | when | Anyone in the RDA | `ballistic/*` † | Clear Range Procedure, red flags and lights, stop-firing communications (procedural) |
| RNG-EX-WN-02 | Weapon found loaded after the unload and proving drill, at the unloading facility or at the armoury | when | Firer, others nearby, armourer | `ballistic/*` † | Unload and prove with a second check (procedural); none passive |
| RNG-EX-WN-03 | Delayed function after a misfire: a hangfire, or a demolition charge that detonates late | when | Anyone approaching the misfire | `explosion/*` † | Misfire waiting times (procedural) |
| RNG-EX-WN-04 | A blind functions when disturbed by grazing, fire, a vehicle or clearance | when | Clearance team, maintenance staff | `explosion/fragmenting_munition` † | Closed impact area fence (passive); EOD procedures (procedural) |
| RNG-EX-WN-05 | Inadvertent initiation of an electric detonator during preparation (radio frequency, static, early connection) | when | Firing party | `explosion/*` † | RF safety distances (Vol 4), shunts and exploder key control |
| RNG-EX-HM-01 | Live round in blank or drill ammunition, or live ammunition in a blank-firing exercise | how much | Anyone the weapon is aimed at | `ballistic/*` † | Ammunition segregation and checks at issue (procedural) |
| RNG-EX-HM-02 | Ammunition above the range's authorised nature or muzzle energy (for example, 8.6 mm on a 25 m range, which Vol 2 Table 2-2 note 7 prohibits) | how much | Beyond the butt, markers | `ballistic/centrefire_rifle` † | Authorised natures in Range Standing Orders (procedural) |
| RNG-EX-HM-03 | Charge NEQ above that used to calculate the safety distance | how much | Firing party, sentries | `explosion/*` † | NEQ checks against the tables (procedural) |

**Incursions: the people change**

| ID | Scenario | Dimension | People at risk | Band | Direct controls (prevent) |
|---|---|---|---|---|---|
| RNG-IN-WR-01 | Unauthorised person (walker, rider, vehicle) inside the RDA during firing, past flags and sentries | where | Unauthorised public | `ballistic/*` † | Sentries, surveillance, fences (Vol 1 Pt 2 para 102); means to stop firing promptly |
| RNG-IN-WR-02 | Authorised person forward of the firing line during firing (to fetch kit, check a target, or in the wrong lane) | where | Firers, range staff | `ballistic/*` † | Firing-line control by the RCO (procedural) |
| RNG-IN-WR-03 | Butt marker exposed above the gallery or mantlet during firing | where | Butt marker | `ballistic/*` † | Gallery and mantlet design (passive); butt communications (procedural) |
| RNG-IN-WR-04 | Person enters a closed impact area, at any time | where | Maintenance staff, troops, unauthorised public | `explosion/fragmenting_munition` † | Demarcation fence and "No Entry to Impact Area" signs (Vol 2 para 2-106) |
| RNG-IN-WR-05 | Person in the backblast area of an anti-tank weapon when it fires (20 m behind the firer, ±45°, Vol 2 Ch 20) | where | Safety supervisor, adjacent troops | `explosion/*` † | Bay walls and siting (passive); backblast checks (procedural) |
| RNG-IN-WR-06 | Person out of cover inside the burst safety distance at a grenade range or demolition | where | Spectators, sentries, firing party | `explosion/fragmenting_munition` † | Protected shelter (passive); sentries (procedural) |
| RNG-IN-WR-07 | Aircraft enters the air danger area during firing | where | Aircrew | `ballistic/*` † | Airspace notification, air sentries, stop firing (procedural) |
| RNG-IN-WN-01 | Range party enters the RDA or impact area before firing has stopped, before "range clear", or before the misfire waiting time has passed | when | Range party | `ballistic/*` or `explosion/*` † | Clear Range Procedure; waiting times (procedural) |
| RNG-IN-HM-01 | More people than planned inside a safety distance, for example spectators at a demonstration (Vol 4 Table 403/3/1) | how much | Spectators | per source | Spectator areas set by safety distance (passive) |

### 4.4 What surfaces: Elevated

| ID | Scenario | Band | Note |
|---|---|---|---|
| RNG-EX-WR-10 | Backsplash beyond the backsplash distance (Vol 2 Table 2-2) | `ballistic/backsplash` † | The eye is the region at risk. The body-region rule ([sif-severity.md](sif-severity.md) §8) may lift it to SIF; the panel decides |
| RNG-IN-WR-08 | Person moves inside the backsplash distance of a hard target (for example, close engagement on live-fire tactical training) | `ballistic/backsplash` † | As above |
| RNG-EX-HM-04 | Weapon or barrel failure at the firer: obstruction, wrong round, overpressure | `explosion/minor_confined` (0.10) | Face, eyes and hands are at risk; the body-region rule may lift it to SIF |
| RNG-EX-WN-06 | Blank fired at close range (gas and debris within the blank safety distance, Vol 4 Table 403/3/1) | `pressure/*` † | Eye and face; no calibrated band |

### 4.5 What surfaces: SIF exposures (conditions, not events)

These carry no PSIF count. They are listed because the risk assessment has to see them; each is the condition its PSIF events come from.

| ID | Exposure | Why it is not an event |
|---|---|---|
| RNG-SX-01 | Firers on the firing point with loaded weapons, within reach of their neighbours' weapons | Planned presence; loaded weapons are live, held by a safety catch and a trigger finger |
| RNG-SX-02 | Butt markers in the gallery during firing, behind the mantlet | Planned presence behind a passive control |
| RNG-SX-03 | Safety staff moving with firers on live-fire tactical training | Planned presence; the WDA template excludes low-probability human error, so this residual sits with the people |
| RNG-SX-04 | EOD operators clearing blinds | Planned presence at live energy is exposure; the blind functioning is the event (RNG-EX-WN-04) |
| RNG-SX-05 | Firing party in a protected shelter within a demolition safety distance | Planned presence behind a passive control |
| RNG-SX-06 | Sentries and wardens at the RDA boundary; public on permitted-access land outside the RDA | Planned presence outside the danger zone |

### 4.6 What does not surface

| Case | Coding | Why |
|---|---|---|
| A prevented incursion: a walker stopped at the boundary by a sentry | Not an event | Separation held. It is worth recording as a control that worked, which the MoD already asks for (DCOP 313 para 22) |
| Firing with the RDA confirmed clear, all rounds inside the template | Not an event | A planned release inside its envelope |
| A weapon proved clear and inspected | No exposure | Zero-energy: the rounds are removed |
| Weapons and ammunition in storage | Exposure only if a release is credible | At-rest. Explosives storage is a separate domain (the DSA 03.OME storage regulations) |
| Noise-induced hearing loss; lead in indoor ranges | Out of scope | Long-term exposure, not an energy transfer event. Impulse noise from a single shot is `radiation_noise/impulse_noise`, Not SIF |
| Heat illness on exercise | Out of the range pilot | `thermal/hot_environment_extreme` is SIF, but not range-specific; it belongs to a training-activity index |

### 4.7 Patterns

- **Wide approaches dominate.** Most range excursions (RNG-EX-WR-01, -02, -06) and the main incursion (RNG-IN-WR-01) usually happen with nobody near. Separation to zero makes them PSIF anyway. This is where a range index will add the most counts, and where the panel will most need to agree the rule ([proposal.md](proposal.md) Annex A raises the same point for a fire in an unoccupied building).
- **Excursions in time are the hidden class.** RNG-EX-WN-02 (found loaded) and RNG-EX-HM-01 (live in blank) involve no discharge and no visible harm. Under the energy-state terms they are live energy where zero-energy was declared, which is the same shape as the back-fed circuit in the worked examples. Report-led coding is likely to miss them; the index names them.
- **Many SIF-class scenarios have only procedural controls.** Muzzle sweep has no direct control at all, and found-loaded has only a procedural one. Unload and prove, misfire waiting times, ammunition checks and the Clear Range Procedure are all procedures. Passive controls (stop butts, baffles, mantlets, bay walls, fences) cover the where-excursions and fixed danger zones well. They cover the when-excursions hardly at all.
- **Impact areas never go to rest.** A blind's fuze can function on disturbance, so the energy is live and the closed impact area is a standing danger zone. Entry is an incursion whether or not firing is in progress.

## 5. Gap: the band table has no ballistic band

Annex B has no band for a projectile fired from a weapon. Today a coder would pick one of three:

| Option | P(serious) | Problem |
|---|---|---|
| `struck/swinging_projectile` | 0.439, Elevated | It covers "swinging load, pressurised release, high-speed projectile"; it under-rates a rifle round |
| `assault/firearm` | 0.720, SIF | The mechanism is violence by another person; the data behind it is mostly handgun assault |
| `explosion/*` | 0.10–0.90 | Industrial descriptions; no fragmentation band, and no link to burst safety distance |

**Proposal:** add a `ballistic` mechanism and two `explosion` bands, provisional until calibrated.

| Mechanism / band | Covers | Provisional class | Anchor |
|---|---|---|---|
| `ballistic/air_weapon` | Air rifles and pistols | Not SIF; the eye is the exception (body-region rule) | UK air rifle limit, 16.3 J (12 ft lb) |
| `ballistic/backsplash` | Fragments and debris thrown back from a target | Elevated | Vol 2 Table 2-2 |
| `ballistic/handgun` | Pistol, rimfire, carbine | SIF | `assault/firearm` 0.72 |
| `ballistic/centrefire_rifle` | Rifle and machine gun | SIF | At least `assault/firearm` 0.72, given the higher muzzle energy |
| `explosion/fragmenting_munition` | Grenades, mortar and HE natures within the burst safety distance | SIF | `explosion/major_catastrophic` 0.90 |
| `explosion/demolition_charge` | Charges by NEQ within the safety distance | SIF | `explosion/significant_industrial` 0.50 or higher, by NEQ and distance |

The anchors are borrowed values, not calibration. The AI model uses the same band table, so the gap affects human and AI coding alike and must be fixed in the model's calibration before a range pilot. The fix is a Spongl Ltd development item.

## 6. Crosswalk: MoD range terms and SIF terms

| MoD term (DSA 03.OME Pt 3) | SIF term | Note |
|---|---|---|
| Weapon Danger Area / Zone (Vol 2 para 2-48) | Envelope (where), for an intentional release | Includes fragments and ricochet; "excludes low probability human and system errors" |
| Range Danger Area / Zone (para 2-38) | Danger zone | The area kept clear of people during firing |
| Clear Range Procedure (para 2-13) | Separation in where and when | A procedural control; "timely cessation of firing" is its active part |
| Impact area; closed impact area (paras 2-25, 2-106) | Danger zone around live energy (blinds) | Standing, not limited to the firing window |
| Burst safety distance (para 2-12); explosive safety distances (Vol 4 Annex C) | Envelope (where) for fragmenting and explosive natures | |
| Backsplash zone (para 2-11) | Envelope (where) for debris thrown back | |
| Backblast area (Vol 2 Ch 20) | Danger zone behind the weapon | Included in the RDA (para 2-110) |
| Incursion (Vol 1 Pt 2 para 99; DCOP 313 para 22) | Incursion | Same meaning: people entering the danger area. A "prevented incursion" is not an event |
| "Un-intended functioning of a weapon system, munition, pyrotechnic or explosive store" (Vol 1 Pt 2 para 9) | Excursion | The MoD reports it with accidents; the SIF rules code it as an event whatever the outcome |
| **Line of Fire**: "an imaginary straight line from the barrel of the weapon delivery system to the target" (Vol 2 para 2-27) | **Not** our line of fire | **Clash.** Our line of fire is the path live energy takes or a foreseeable release would take; for a weapon that is the cone of fire plus ricochet, so the WDA. Range coders must be told which is meant |
| "Authorised but unprotected persons" (Vol 1 Pt 2 para 100) | People in the target envelope without a passive control | Separates planned exposure (markers behind a mantlet) from incursion |

## 7. Pilot plan

1. **Scope:** four range types: the 25 m barrack range (Vol 2 Ch 6), the grenade range (Ch 18), live-fire tactical training (Ch 15) and demolitions (Vol 4).
2. **Fix the band gap** (section 5) in the band table and the AI model's calibration.
3. **Build and review the index** for the four types, with the panel and a range safety specialist (an RCO or range warden), using sections 3 and 4 as the starting set.
4. **Back-test.** Take 12 months of range occurrence and incident reports and code each against the index. Record the scenario match, closest approach, control state and determination.
5. **Measure:**
   - coverage: the share of reports that match a scenario (target 90%); unmatched reports become new scenarios;
   - silent scenarios: SIF-class scenarios with no reports, which are either rare or not reported;
   - control failure by control: which direct and procedural controls fail most;
   - reliability: agreement between coders with and without the index (the panel's kappa).
6. **Hand-over:** each activity's risk assessment cites its scenario IDs. The direct controls against SIF-class scenarios become the critical-control list for verification.

## 8. Open questions

1. **Wide-approach PSIF.** Does the client accept PSIF counts for rounds leaving the danger area with nobody near? If not, the index should still list them, and the panel's scoping rule should say why they are excluded.
2. **Procedural-only controls.** For scenarios with no direct control (muzzle sweep, found loaded), is the control state always "failed or absent" by definition, as EEI's direct-control test implies? Or should a verified procedural control count as protected on ranges?
3. **Data access.** The back-test needs range occurrence reports. Whose are they, and is "prevented incursion" recorded in a usable form?
4. **Domain boundary.** Explosives storage, transport of ammunition and heat illness are left out of the range pilot. Should they be separate indexes or later rows?

## References

DOSR (Defence OME Safety Regulator) (2019a). *DSA 03.OME Part 3 (JSP 403) Volume 1 Part 2: Defence Code of Practice and guidance notes for ranges*. Ministry of Defence, October 2019. GOV.UK: DSA03 OME part 3: ranges.

DOSR (2019b). *DSA 03.OME Part 3 (JSP 403) Volume 4: demolitions, explosive ordnance disposal and battle simulation*. Ministry of Defence, April 2019.

DOSR (2020). *DSA 03.OME Part 3 Volume 1 Part 1: Defence Code of Practice and guidance notes for ranges*. Ministry of Defence, June 2020.

DOSR (2026a). *Defence Code of Practice (DCOP) and guidance notes for ranges: Volume 2*, version 2.2. Ministry of Defence, updated September 2026.

DOSR (2026b). *DSA 03.OME Part 3 DCOP 313: Fieldcraft Training Areas (FTA)*. Ministry of Defence, May 2026.

ICMM (2015). *Health and safety critical control management: good practice guide*. International Council on Mining and Metals.

Ministry of Defence (n.d.). *JSP 403: Handbook of Defence Land Ranges Safety* (collection page, withdrawn content). GOV.UK.

Companion papers: [sif-definition.md](sif-definition.md), [sif-severity.md](sif-severity.md), [sif-controls-precursors.md](sif-controls-precursors.md), [sif-counterfactual.md](sif-counterfactual.md), [proposal.md](proposal.md).
