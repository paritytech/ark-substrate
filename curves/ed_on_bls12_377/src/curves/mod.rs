use ark_ff::MontFp;
use ark_scale::{hazmat::ArkScaleProjective, ArkScale};
use ark_std::{marker::PhantomData, vec::Vec};
use codec::{Decode, Encode};
use sp_ark_models::{
    twisted_edwards::{Affine, MontCurveConfig, Projective, TECurveConfig},
    CurveConfig,
};

use crate::{fq::Fq, fr::Fr};

#[cfg(test)]
mod tests;

pub type EdwardsAffine<H> = Affine<EdwardsConfig<H>>;
pub type EdwardsProjective<H> = Projective<EdwardsConfig<H>>;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct EdwardsConfig<H: HostFunctions>(PhantomData<fn() -> H>);

pub trait HostFunctions: 'static {
    fn ed_on_bls12_377_msm(bases: Vec<u8>, scalars: Vec<u8>) -> Result<Vec<u8>, ()>;
    fn ed_on_bls12_377_mul_projective(base: Vec<u8>, scalar: Vec<u8>) -> Result<Vec<u8>, ()>;
}

impl<H: HostFunctions> CurveConfig for EdwardsConfig<H> {
    type BaseField = Fq;
    type ScalarField = Fr;

    /// COFACTOR = 4
    const COFACTOR: &'static [u64] = &[4];

    /// COFACTOR_INV =
    /// 527778859339273151515551558673846658209717731602102048798421311598680340096
    const COFACTOR_INV: Fr =
        MontFp!("527778859339273151515551558673846658209717731602102048798421311598680340096");
}

impl<H: HostFunctions> TECurveConfig for EdwardsConfig<H> {
    /// COEFF_A = -1
    const COEFF_A: Fq = MontFp!("-1");

    /// COEFF_D = 3021
    const COEFF_D: Fq = MontFp!("3021");

    /// Generated randomly
    const GENERATOR: Affine<Self> = Affine::<Self>::new_unchecked(GENERATOR_X, GENERATOR_Y);

    type MontCurveConfig = EdwardsConfig<H>;

    /// Multiplication by `a` is just negation.
    /// Is `a` 1 or -1?
    #[inline(always)]
    fn mul_by_a(elem: Self::BaseField) -> Self::BaseField {
        -elem
    }

    fn msm(
        bases: &[Affine<Self>],
        scalars: &[<Self as CurveConfig>::ScalarField],
    ) -> Result<Projective<Self>, usize> {
        let bases: ArkScale<&[Affine<Self>]> = bases.into();
        let scalars: ArkScale<&[<Self as CurveConfig>::ScalarField]> = scalars.into();

        let result = H::ed_on_bls12_377_msm(bases.encode(), scalars.encode()).unwrap();

        let result = <ArkScaleProjective<Projective<EdwardsConfig<H>>> as Decode>::decode(
            &mut result.as_slice(),
        );
        result.map_err(|_| 0).map(|res| res.0)
    }

    fn mul_projective(base: &Projective<Self>, scalar: &[u64]) -> Projective<Self> {
        let base: ArkScaleProjective<Projective<Self>> = (*base).into();
        let scalar: ArkScale<&[u64]> = scalar.into();

        let result = H::ed_on_bls12_377_mul_projective(base.encode(), scalar.encode()).unwrap();

        let result =
            <ArkScaleProjective<Projective<Self>> as Decode>::decode(&mut result.as_slice());
        result.unwrap().0
    }

    fn mul_affine(base: &Affine<Self>, scalar: &[u64]) -> Projective<Self> {
        let base: Projective<Self> = (*base).into();
        let base: ArkScaleProjective<Projective<Self>> = base.into();
        let scalar: ArkScale<&[u64]> = scalar.into();

        let result = H::ed_on_bls12_377_mul_projective(base.encode(), scalar.encode()).unwrap();

        let result =
            <ArkScaleProjective<Projective<Self>> as Decode>::decode(&mut result.as_slice());
        result.unwrap().0
    }
}

impl<H: HostFunctions> MontCurveConfig for EdwardsConfig<H> {
    /// COEFF_A = 0x8D26E3FADA9010A26949031ECE3971B93952AD84D4753DDEDB748DA37E8F552
    ///         = 3990301581132929505568273333084066329187552697088022219156688740916631500114
    const COEFF_A: Fq =
        MontFp!("3990301581132929505568273333084066329187552697088022219156688740916631500114");

    /// COEFF_B = 0x9D8F71EEC83A44C3A1FBCEC6F5418E5C6154C2682B8AC231C5A3725C8170AAD
    ///         = 4454160168295440918680551605697480202188346638066041608778544715000777738925
    const COEFF_B: Fq =
        MontFp!("4454160168295440918680551605697480202188346638066041608778544715000777738925");

    type TECurveConfig = EdwardsConfig<H>;
}

/// GENERATOR_X =
/// 4497879464030519973909970603271755437257548612157028181994697785683032656389,
const GENERATOR_X: Fq =
    MontFp!("4497879464030519973909970603271755437257548612157028181994697785683032656389");

/// GENERATOR_Y =
/// 4357141146396347889246900916607623952598927460421559113092863576544024487809
const GENERATOR_Y: Fq =
    MontFp!("4357141146396347889246900916607623952598927460421559113092863576544024487809");
