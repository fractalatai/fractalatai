# The Energy Wheel

## The core idea

Every workplace injury is caused by unwanted energy transfer — energy moving from a source to a person's body in a way that causes harm. This is William Haddon's energy transfer theory from 1973, operationalised by Matthew Hallowell in *Energy-Based Safety* (2024).

The insight is simple: instead of asking "how bad could this be?" (a subjective question), ask "how much energy is involved?" (a measurable question). A fall from 2 metres involves roughly 1,500 joules. A fall from 6 metres involves roughly 4,400 joules. The energy predicts the severity far better than any committee assessment.

## The ten energy types

Hallowell's Energy Wheel organises all workplace hazards into ten categories:

| # | Energy type | Everyday examples | What determines severity |
| --- | --- | --- | --- |
| 1 | **Gravity** | Falls from height, falling objects, structural collapse | Height and mass — every metre adds ~750 J for a 75 kg person |
| 2 | **Motion** (kinetic) | Vehicles, mobile plant, projectiles | Mass and speed — double the speed means 4× the energy |
| 3 | **Mechanical** | Rotating equipment, presses, conveyors, shearing | Rotational speed, torque, whether guarding is present |
| 4 | **Electrical** | Power lines, switchgear, arc flash | Voltage and available current |
| 5 | **Pressure** | Hydraulic systems, compressed gas, steam, excavations | Pressure and volume of the contained system |
| 6 | **Thermal** | Hot surfaces, molten material, fire, welding | Temperature, duration of contact, body area exposed |
| 7 | **Chemical** | Toxic gas, corrosives, oxygen-displacing agents | Concentration, volume, exposure route |
| 8 | **Radiation** | X-ray, gamma, UV, laser | Dose and type of radiation |
| 9 | **Sound** | Explosions, pneumatic tools | Blast overpressure |
| 10 | **Biological** | Pathogens, venomous animals | Rarely SIF alone; exception: anaphylaxis, sepsis |

Most workplace fatalities cluster in just a few of these: gravity (falls), motion (vehicles), mechanical (machinery), electrical, and pressure. Hallowell's research identified 13 specific high-energy hazard scenarios — "Stuff That Kills You" (STKY) — that account for roughly 75% of all SIF events.

## From energy to severity

The link between energy and severity is not guesswork — it comes from decades of OSHA and RIDDOR injury data. For each energy type, empirical data shows the relationship between energy magnitude and injury outcomes:

**Gravity example:**
- Below 1.8 m (6 ft): mostly first aid and medical treatment
- 1.8–3 m: serious injuries become common
- Above 6 m: fatalities dominate the distribution
- The SIF threshold is roughly 1.8 m — the energy at this height (~1,300 J for a 75 kg person) is enough to cause fatal head injuries on a hard surface

**Electrical example:**
- Below 50 V: painful but rarely dangerous
- 50–240 V: can be fatal (domestic/industrial mains)
- Above 1,000 V: almost always fatal without protection
- The SIF threshold is roughly 50 V

These thresholds are not arbitrary cut-offs — they are derived from the statistical point where the outcome distribution shifts from "mostly non-serious" to "substantial probability of SIF."

## Why this replaces opinion with physics

On a probability-impact grid, two assessors might disagree about whether a particular fall is "high" or "medium" severity. With the Energy Wheel, the question is: how high was the fall? If it was 6 metres, the gravitational potential energy is 4,400 J, and the severity distribution is determined by calibrated empirical data. There is nothing to disagree about.

The assessor's job shifts from guessing at severity to describing the scenario: what energy type, what magnitude, what the receiving environment looks like. The system handles the rest. This is why the [SIF classifier](../papers/from-pigs-to-probability-curves.md) separates "what happened" (Stage 1) from "how much energy" (Stage 2) — the first is a description, the second is a calculation.

## Further reading

- Hallowell, M.R. (2024). *Energy-Based Safety*. CRC Press/Routledge.
- Haddon, W. (1973). Energy Damage and the Ten Countermeasure Strategies. *Human Factors*, 15(4).
- CHASNZ. Energy Wheel. https://chasnz.org/energy-wheel
