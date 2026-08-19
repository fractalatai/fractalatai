---
session: SIPmath Engine
status: active
opened: 2026-08-19
---

# Session: SIPmath Engine (ACTIVE)

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
