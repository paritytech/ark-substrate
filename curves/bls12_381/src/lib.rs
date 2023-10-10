//! This library implements the BLS12_381 curve generated by [Sean Bowe](https://electriccoin.co/blog/new-snark-curve/).
//! The name denotes that it is a Barreto--Lynn--Scott curve of embedding degree
//! 12, defined over a 381-bit (prime) field.
//! This curve was intended to replace the BN254 curve to provide a higher
//! security level without incurring a large performance overhead.
//!
//!
//! Curve information:
//! * Base field: q = 4002409555221667393417789825735904156556882819939007885332058136124031650490837864442687629129015664037894272559787
//! * Scalar field: r =
//!   52435875175126190479447740508185965837690552500527637822603658699938581184513
//! * valuation(q - 1, 2) = 1
//! * valuation(r - 1, 2) = 32
//! * G1 curve equation: y^2 = x^3 + 4
//! * G2 curve equation: y^2 = x^3 + Fq2(4, 4)

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(
    warnings,
    unused,
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms
)]
#![allow(clippy::result_unit_err)]
#![forbid(unsafe_code)]

pub mod curves;

pub use ark_bls12_381::{fq, fq::*, fq12, fq12::*, fq2, fq2::*, fq6, fq6::*, fr, fr::*};
pub use curves::*;

use ark_scale::ark_serialize::{Compress, Validate};

#[cfg(feature = "scale-no-compress")]
const SCALE_COMPRESS: Compress = Compress::No;
#[cfg(not(feature = "scale-no-compress"))]
const SCALE_COMPRESS: Compress = Compress::Yes;

#[cfg(feature = "scale-no-validate")]
const SCALE_VALIDATE: Validate = Validate::No;
#[cfg(not(feature = "scale-no-validate"))]
const SCALE_VALIDATE: Validate = Validate::Yes;

/// SCALE codec usage settings.
///
/// Determines whether compression and validation has been enabled for SCALE codec
/// with respect to ARK related types.
pub const SCALE_USAGE: u8 = ark_scale::make_usage(SCALE_COMPRESS, SCALE_VALIDATE);

type ArkScale<T> = ark_scale::ArkScale<T, SCALE_USAGE>;
