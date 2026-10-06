# Review: how we label obligations and liberties (2026-10-06)

Jason asked for a root-and-branch review: "the rulings are becoming complex and the prompt equally complex… I feel this could be lots simpler and cheaper." This review measures where the complexity and cost go, and proposes a simpler pipeline.

## 1. What the labels are for

The model of obligations and liberties serves four consumers. Their needs differ a lot in precision:

| Consumer | Needs | Precision needed |
|---|---|---|
| **Making verdict** (`is_making`, law-level) | does any provision give an **active** actor an Obligation (a Duty/Responsibility)? | high, but at law level: one correct Duty is enough |
| **Holder lists** (duty/rights/responsibility/power holders) | active actor + held type, per provision | high |
| **Correlatives** ("my rights", what can be imposed on me) | counterparty / beneficiary + act | medium–high |
| **Purpose profile** (legal#172) and future DRRP gating (#76) | one purpose per provision, aggregated to law-level shares ≥ 0.05 | **low per provision**: shares tolerate noise |

So the precision that matters is: **does the provision create a relation, who holds it, and who is owed it.** Purpose matters as a gate and a law-level distribution, not as a precise per-provision fact.

## 2. What we built

- **One definitive prompt answers everything for every provision:** relation, raw_type, purpose (18 values), and per actor position / holds / inferred / act, with stem, referenced and applying-provision context.
- **The prompt has grown** from v1.0 to v1.4 in two days: ~42K characters (~10K tokens). It holds about 45 special-case rules, most added to settle disagreements.
- **Training labels:** Gemini over 6,959 provisions, a second model (GPT-5.5, then GPT-5.4-mini), and a Claude referee on the disputes.

## 3. Where the complexity and cost actually go (measured)

**a) Most provisions don't need an LLM to know there is no relation.**
- 234,363 substantive live provisions in the hub. **75,754 (32%) contain no duty word** (shall, must, may, is entitled, duty, power to, is required, is to…) in their own text or their stem.
- In the labelled sample, only **19 of 841** such provisions are relations (2%). A free regex prefilter is ~98% right.
- A further 119,107 provisions are amendment, structural or out of scope, which we already skip.

**b) Purpose and relation are nearly the same decision.**
- Machinery purposes are relation `yes` in only 21 of 1,357 labelled provisions (1.5%); `Procedure+Detail` in **0 of 1,029**.
- So gating on purpose would skip **28%** of the duty-word provisions and lose **0.5%** of real relations (21 of 4,198).

**c) Most disagreement, and most of the rules, are about purpose and machinery edge cases.**
- Of 973 model disputes in batches 1–2, **565 (58%) involve purpose**; 191 are purpose only.
- Of the ~45 prompt rules, roughly two-thirds settle purpose precedence or relation-`no` edge cases: commencement and notification, laying before Parliament, money clauses, designation in definitions, electronic delivery, savings and deeming, timing and discharge details, functions lists, appeals, exemption powers.
- Those rarely change any consumer output above: a Making verdict, a holder list or a correlative.

**d) The second model buys little for training labels.**
- Against the referee on disputed provisions, Gemini scored relation 94% and purpose 90% (exact 71%). GPT-5.5 scored 27% exact; GPT-5.4-mini 36%.
- The second model's value is flagging Gemini's ~25% disputed cases, and the referee sides with Gemini about 70% of the time.
- An SLM trained on 7K labels tolerates a few percent label noise. Precision matters far more in the **evaluation and gold** set.

**e) The pipeline guards are stale.** The parser's purpose skip-gates still use the old purpose vocabulary (#76: "move DRRP gating and scope onto the published purpose"). Nothing yet uses the new purpose to avoid work.

## 4. Root causes

1. **Coupling.** One prompt answers five questions at once, so every edge case needs a rule that touches several fields, and the rules interact. Example: the consistency rule plus its three exceptions (transitional, commencement, exemption powers).
2. **Uniform effort.** Every provision gets the full, expensive treatment, whether it's a definition, a commencement clause or a duty.
3. **Precision where it isn't needed.** Per-provision purpose precision and machinery minutiae are policed as strictly as holders and correlatives.
4. **Perfecting training labels instead of the evaluation set.** Two models plus a referee on 7K training rows, while gold v2 (the benchmark) is still waiting.

## 5. Proposal: a staged pipeline, simplest first

| Stage | What | Cost | Removes |
|---|---|---|---|
| **0. Structural and duty-word filter** (regex, free) | scope ≠ substantive → skip. No duty word in text or stem → relation `no`, no actors labelled; purpose by stage 1. | free | ~32% of substantive provisions, plus 119K non-substantive |
| **1. Purpose classifier** (cheap: embeddings + linear model, or the SLM) | trained on the ~7K purpose labels we already have. Machinery / `Procedure+Detail` → relation `no`, except "may"-powers to commence, exempt or a time-limited transitional power, which go to stage 2. Feeds the #76 guards and the purpose profile. | ~free at inference | a further ~28% of duty-word provisions; purpose is no longer in the LLM's job |
| **2. Relation + actors** (SLM; LLM only when the SLM is unsure) | for operative provisions only: relation, holder(s) (stem, referenced, applying), counterparty/beneficiary + act. **A short prompt** (Hohfeld positions, the act test, holder resolution) with **no purpose rules**. | SLM ~free; LLM on the low-confidence slice | the purpose/machinery rules and ~58% of disputes |
| **3. Referee (Claude)** | only (a) the evaluation/gold set and (b) a QA sample per release, not every training dispute | small, targeted | — |

**What it changes:**
- **The prompt splits into two short ones:** a relation/actor prompt (~⅓ of today's) and a purpose definition list for the classifier. Most of the ~45 special cases become **defaults** ("not an operative duty or power → relation `no`, purpose from the classifier") instead of LLM rules.
- **LLM and SLM calls fall by ~half or more** on the corpus (stage 0 + stage 1), before the SLM takes over the bulk.
- **Rule governance:** a new rule enters the relation prompt only if it can change a verdict, holder or correlative. Purpose edge cases go to the classifier's label set, and its errors are tolerated as noise in law-level shares.

## 6. What to do with the work already done

- **Training labels:** keep Gemini's 6,959 v1.3 labels and the refereed batches 1–2 (1,357 + 1,635). **Stop the second-model pass for batches 3–5.** That saves the OpenAI spend; the referee patterns already tell us where Gemini is weak (trigger-condition beneficiaries, detail-vs-relation, class-definition items) for targeted QA.
- **The v1.4 refresh** (489 Gemini labels, ~$2): run it only if we keep the single combined prompt. Under the staged design, purpose rulings 2–15 move to the classifier's training data instead.
- **Purpose classifier:** train on the 6,959 purpose labels (cheap, local), then measure it against the referee's purpose decisions.
- **Retrain the SLM on relation + actors only** (positions, holds, act), not purpose.
- **Gold v2:** this is where two-model-plus-referee rigour belongs. It's the published benchmark data.

## 7. Risks and checks

- **Stage 0 misses** (2% of no-duty-word provisions are relations): mostly list items whose stem has the duty word. Those are kept, because the stem counts. Measure on the 7K labels before switching.
- **Stage 1 gate errors:** measured at 0.5% of relations with Gemini's purpose. Re-measure with the trained classifier, and send any gated provision that has a duty word and a named actor to stage 2 if the classifier is unsure.
- **Purpose profile drift:** compare law-level purpose shares from the classifier against the LLM labels on the sample.

## 8. Decisions for Jason

1. Adopt the staged pipeline (0 → 1 → 2 → targeted referee)?
2. Stop the second-model labelling after batch 2, and redirect referee effort to gold v2 and QA?
3. Split purpose out of the relation prompt into a cheap classifier, trained on the labels we have?
4. Hold the v1.4 Gemini refresh (~$2) until 1–3 are decided?
