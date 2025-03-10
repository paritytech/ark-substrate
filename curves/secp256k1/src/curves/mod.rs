use ark_secp256k1::Config as ArkConfig;
use ark_ff::MontFp;
use ark_models_ext::{
    models::CurveConfig,
    short_weierstrass::{self, SWCurveConfig},
};
use ark_std::marker::PhantomData;

/// G_GENERATOR_X =
/// 55066263022277343669578718895168534326250603453777594175500187360389116729240
pub const G_GENERATOR_X: <ArkConfig as CurveConfig>::BaseField =
    MontFp!("55066263022277343669578718895168534326250603453777594175500187360389116729240");

/// G_GENERATOR_Y =
/// 32670510020758816978083085130507043184471273380659243275938904335757337482424
pub const G_GENERATOR_Y: <ArkConfig as CurveConfig>::BaseField =
    MontFp!("32670510020758816978083085130507043184471273380659243275938904335757337482424");


pub type Affine<H> = short_weierstrass::Affine<Secp256k1Config<H>>;
pub type Projective<H> = short_weierstrass::Projective<Secp256k1Config<H>>;

#[derive(Clone, Copy)]
pub struct Secp256k1Config<H: CurveHooks>(PhantomData<fn() -> H>);

/// Hooks for *Secp256k1*.
pub trait CurveHooks: 'static + Sized {
    /// Short Weierstrass multi scalar multiplication.
    fn secp256k1_msm(
        bases: &[Affine<Self>],
        scalars: &[<Secp256k1Config<Self> as CurveConfig>::ScalarField],
    ) -> Result<Projective<Self>, ()>;

    /// Short Weierstrass projective multiplication.
    fn secp256k1_mul_projective(
        base: &Projective<Self>,
        scalar: &[u64],
    ) -> Result<Projective<Self>, ()>;
}

impl<H: CurveHooks> CurveConfig for Secp256k1Config<H> {
    const COFACTOR: &'static [u64] = <ArkConfig as CurveConfig>::COFACTOR;
    const COFACTOR_INV: Self::ScalarField = <ArkConfig as CurveConfig>::COFACTOR_INV;

    type BaseField = <ArkConfig as CurveConfig>::BaseField;
    type ScalarField = <ArkConfig as CurveConfig>::ScalarField;
}


impl<H: CurveHooks> SWCurveConfig for Secp256k1Config<H> {
    const COEFF_A: Self::BaseField = <ArkConfig as SWCurveConfig>::COEFF_A;
    const COEFF_B: Self::BaseField = <ArkConfig as SWCurveConfig>::COEFF_B;

    const GENERATOR: Affine<H> = Affine::<H>::new_unchecked(G_GENERATOR_X, G_GENERATOR_Y);


    /// Multi scalar multiplication jumping into the user-defined `secp256k1_msm` hook.
    ///
    /// On any internal error returns `Err(0)`.
    #[inline(always)]
    fn msm(bases: &[Affine<H>], scalars: &[Self::ScalarField]) -> Result<Projective<H>, usize> {
        H::secp256k1_msm(bases, scalars).map_err(|_| 0)
    }

    /// Projective multiplication jumping into the user-defined `secp256k1_mul_projective` hook.
    ///
    /// On any internal error returns `Projective::zero()`.
    #[inline(always)]
    fn mul_projective(base: &Projective<H>, scalar: &[u64]) -> Projective<H> {
        H::secp256k1_mul_projective(base, scalar).unwrap_or_default()
    }
}
