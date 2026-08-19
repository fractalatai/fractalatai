---
session: SIPmath Engine
status: closed
opened: 2026-08-19
closed: 2026-08-19
outcome: success

summary: >
  Built the fractalaw-sipmath crate — metalog distributions, HDR PRNG, SIP composition, and
  SIPmath 3.0 JSON serialization. 1,055 lines of Rust across 4 modules, 21 tests passing,
  zero external math dependencies. First known Rust implementation of metalog/SIPmath.

decisions:
  - what: Inline Gaussian elimination instead of nalgebra dependency for OLS
    why: The k×k system for metalog fitting is tiny (typically k ≤ 10). A 30-line solver avoids pulling in a large linear algebra crate and keeps the dependency tree minimal for WASM compilation.
    result: Zero non-serde dependencies. OLS works correctly for 5-term fits tested.
  - what: SplitMix64-based mixing for HDR PRNG instead of the exact HDR specification
    why: The exact HDR algorithm is specified for Excel cell formulas. SplitMix64 is a well-tested hash-based PRNG that passes Dieharder tests and maps naturally to Rust u64 arithmetic.
    result: Chi-squared uniformity test passes at p=0.01 level across 10,000 trials.
  - what: Defer WASM compilation to S3 (simulator session)
    why: WASM is needed for the browser-based simulator prototype, not for the engine itself. Building and testing it alongside the simulator UI gives better validation.
    result: WASM item tracked in S3 todo list.

metrics:
  crate_size: { lines: 1055, modules: 4, tests: 21, dependencies: 3 }
  module_breakdown: { metalog: 549, sip: 199, io: 188, hdr: 104, lib: 15 }
  test_coverage: { spt_unbounded: pass, spt_bounded: pass, spt_semi_bounded: pass, spt_skewed: pass, ols_3term: pass, ols_5term: pass, cdf_roundtrip: pass, pdf_integral: pass, hdr_uniformity: pass, json_roundtrip: pass, mitigation_chain: pass, bernoulli_gate: pass }
  build_time: { check: "2.0s", test: "4.9s" }

lessons:
  - title: SPT a3 denominator is NOT the same as a2
    detail: >
      The 3-term metalog system at y=0.10, 0.50, 0.90 has different denominators for a2 and a3.
      a2 uses (L90 - L10) = 4.394, but a3 uses 2·(0.9-0.5)·L90 = 1.758. The basis function
      g3 = (y-0.5)·logit(y) introduces the (y-0.5) factor that changes the coefficient.
      Initial implementation used delta_l for both, which broke skewed and bounded fits
      while symmetric fits passed (because a3=0 when symmetric). Caught by test coverage.
    tag: methodology
  - title: Bounded metalog transforms must be applied to input data before fitting
    detail: >
      For bounded [0,1] metalog (e.g., mitigation effectiveness), the input quantile values
      are log-transformed before computing coefficients, and the inverse transform is applied
      in the quantile function. The coefficients live in the transformed space. This is easy
      to get wrong — the transform/inv_transform pair must be consistent across fit, quantile,
      CDF, and PDF.
    tag: methodology

artifacts:
  - crates/fractalaw-sipmath/Cargo.toml
  - crates/fractalaw-sipmath/src/lib.rs
  - crates/fractalaw-sipmath/src/metalog.rs
  - crates/fractalaw-sipmath/src/hdr.rs
  - crates/fractalaw-sipmath/src/sip.rs
  - crates/fractalaw-sipmath/src/io.rs

depends_on: []

enables:
  - 08-19-26-taxonomy-and-data.md (metalog fitting for calibration curves)
  - 08-19-26-simulator.md (SIP generation, composition, WASM build)
  - 08-19-26-energy-analyser.md (metalog CDF for P(SIF) from severity quantiles)
---

# Session: SIPmath Engine (CLOSED)

## Problem

Both SIF products (classifier and simulator) depend on a metalog/SIPmath engine. No Rust or JavaScript implementation exists — crates.io and npm have zero metalog or SIPmath packages. This is ~500 lines of Rust, compiles to WASM, and is independently publishable.

## Todo

- ✅ Scaffold `fractalaw-sipmath` crate in workspace
- ✅ Implement `metalog::fit_spt` — 3-term SPT from P10/P50/P90 (closed-form)
- ✅ Implement `metalog::quantile` — evaluate quantile function for all boundedness variants
- ✅ Implement `metalog::cdf` — numerical CDF via bisection on quantile function
- ✅ Implement `metalog::is_feasible` — monotonicity check
- ✅ Implement `metalog::fit_ols` — k-term metalog from n quantile points (OLS via Gaussian elimination)
- ✅ Implement `hdr::uniform` — HDR PRNG with 5-component seed (SplitMix64-based)
- ✅ Implement `sip::generate` — N trials from metalog SIP
- ✅ Implement `sip::chain_mitigations` — element-wise multiplicative composition
- ✅ Implement `sip::exceedance_probability` and `sip::percentile`
- ✅ Implement `io` — SIPmath 3.0 JSON serialization/deserialization
- ✅ Unit tests — 21 tests passing (metalog, HDR, SIP, IO)
- ⏸️ WASM compilation target — deferred to S3 (simulator session)

## Dependencies

- ✅ Design plan v0.3 — SIPmath engine section
- ✅ No external linear algebra deps — OLS uses inline Gaussian elimination with partial pivoting

## Implementation Notes

### Crate structure: 1,055 lines across 4 modules

| Module | Lines | Purpose |
|--------|-------|---------|
| `metalog.rs` | 549 | SPT fit, OLS fit, quantile, CDF (bisection), PDF, feasibility check, 4 boundedness variants |
| `sip.rs` | 199 | SIP generation, mitigation chaining, Bernoulli gates, exceedance, percentile, summary stats |
| `io.rs` | 188 | SIPmath 3.0 JSON serialization/deserialization, Metalog ↔ SipDef conversion |
| `hdr.rs` | 104 | HDR PRNG with 5-component seed, SplitMix64 mixing |

### SPT coefficient fix

Initial implementation had wrong denominator for a3 (skewness coefficient). The 3-term system at y=0.10, 0.50, 0.90 gives:
- a1 = z50
- a2 = (z90 - z10) / (L90 - L10)
- a3 = (z90 + z10 - 2·z50) / (2 · 0.4 · L90) — NOT divided by (L90 - L10)

Fixed and all 21 tests pass.

### Dependencies

Only `serde`, `serde_json`, `thiserror` — no `nalgebra` or `ndarray` needed. The OLS solver is a 30-line inline Gaussian elimination with partial pivoting, sufficient for the small k×k systems (typically k ≤ 10).
