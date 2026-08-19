//! Metalog distributions, HDR PRNG, and SIPmath 3.0 standard.
//!
//! Implements the mathematical foundation for uncertainty arithmetic:
//! - **Metalog distributions** (Keelin 2016): fit continuous distributions from
//!   a handful of quantile estimates. Closed-form quantile function, no iterative solvers.
//! - **HDR PRNG** (Hubbard Decision Research): reproducible, multi-seed uniform random
//!   number generator for Monte Carlo trials.
//! - **SIP composition**: element-wise arithmetic on trial arrays — distributions
//!   that work like numbers.
//! - **SIPmath 3.0 JSON**: standard serialization format for portable uncertainty data.

pub mod hdr;
pub mod io;
pub mod metalog;
pub mod sip;
