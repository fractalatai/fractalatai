# Duty Significance Rating: Prioritising Compliance at Scale

## The Problem

A compliance officer responsible for ESH (Environment, Safety, Health) manages hundreds of applicable laws — each containing dozens to hundreds of individual duties. Not all duties are equal. The general employer duty to "ensure the health, safety and welfare of all employees" demands fundamentally more attention than the duty to "allow an inspector access to premises". But today, every duty looks the same in the register.

## What We Built

Fractalaw rates every statutory Obligation on five significance dimensions — automatically, at scale, for pennies.

**40,468 duties** across **553 UK laws** are rated on:

- **Gravity** — is health, safety, or life at stake, or is this administrative?
- **Scope** — does this duty apply to every employer, or to one specific person?
- **Strength** — is this an absolute duty, or qualified by "so far as is reasonably practicable"?
- **Hierarchy** — is this a foundational general duty, or a sub-paragraph in a schedule?

Each dimension is rated HIGH / MEDIUM / LOW by a purpose-built language model, then combined into an overall significance rating per duty and per law.

## What This Means for Compliance Officers

**At the law level**: "Which of my 274 laws need attention first?" Laws are ranked by a score that balances the importance of their duties with the volume of compliance work they represent. The Construction (Design and Management) Regulations 2015 ranks 7th out of 553 — focused, high-gravity safety duties. The Health and Safety at Work Act ranks 70th — a foundational statute, but its general duties are diluted by enforcement and procedural provisions.

**At the provision level**: "Which duties in this law need my attention?" Within any law, duties are sorted by significance. The critical safety obligations surface to the top; the notification and record-keeping duties drop to the bottom.

**At the Part level**: For large Acts like HSWA, the system breaks down significance by Part. Part I (General Duties) contains 31 HIGH-significance duties. Part IV (Miscellaneous) contains zero. A compliance officer can focus where the duties are.

## What Makes This Different

No existing system rates the inherent significance of individual statutory duties. Compliance risk frameworks rate organisational breach risk. RegTech platforms extract and classify obligations. We rate the duty itself — how important is this obligation, regardless of who you are or how well you comply.

The five-dimension approach means customers can re-weight dimensions to match their risk appetite. A construction company may weight gravity highest. A financial services firm may weight scope. The raw dimensions are preserved, not just the overall score.

## By the Numbers

| | |
|---|---|
| Duties rated | 40,468 |
| Laws covered | 553 |
| Dimensions per duty | 5 |
| Rating time (full corpus) | 65 minutes |
| Total cost | ~$5 |
| Accuracy on benchmarks | 3/4 provisions, 3/3 laws |

The system runs on a fine-tuned 4-billion parameter language model at 18 duties per second. A new customer's 274-law register can be fully rated in under an hour.

## Integration

Significance data flows automatically through the existing Zenoh publish pipeline. Sertantai displays per-law rankings, per-provision filtering, and per-Part breakdowns in the compliance register — giving compliance officers the prioritisation signal they have always needed but never had.
