use crate::{
    util::{
        read_g1_compressed, read_g1_uncompressed, serialize_fq, EncodingFlags, G1_SERIALIZED_SIZE,
    },
    ArkScale, CurveHooks,
};

use ark_bls12_381::g1::Config as ArkConfig;
use ark_ff::PrimeField;
use ark_models_ext::{
    bls12,
    bls12::Bls12Config,
    short_weierstrass::{Affine, Projective, SWCurveConfig},
    AffineRepr, CurveConfig, Group,
};
use ark_scale::{
    ark_serialize::{Compress, SerializationError, Validate},
    hazmat::ArkScaleProjective,
    scale::{Decode, Encode},
};
use ark_std::{
    io::{Read, Write},
    marker::PhantomData,
    ops::Neg,
    One,
};

pub use ark_bls12_381::g1::{BETA, G1_GENERATOR_X, G1_GENERATOR_Y};

pub type G1Affine<H> = bls12::G1Affine<crate::Config<H>>;
pub type G1Projective<H> = bls12::G1Projective<crate::Config<H>>;

#[derive(Clone, Copy)]
pub struct Config<H: CurveHooks>(PhantomData<fn() -> H>);

impl<H: CurveHooks> CurveConfig for Config<H> {
    type BaseField = <ArkConfig as CurveConfig>::BaseField;
    type ScalarField = <ArkConfig as CurveConfig>::ScalarField;

    const COFACTOR: &'static [u64] = <ArkConfig as CurveConfig>::COFACTOR;
    const COFACTOR_INV: Self::ScalarField = <ArkConfig as CurveConfig>::COFACTOR_INV;
}

impl<H: CurveHooks> SWCurveConfig for Config<H> {
    const COEFF_A: Self::BaseField = <ArkConfig as SWCurveConfig>::COEFF_A;
    const COEFF_B: Self::BaseField = <ArkConfig as SWCurveConfig>::COEFF_B;

    const GENERATOR: Affine<Self> = Affine::<Self>::new_unchecked(G1_GENERATOR_X, G1_GENERATOR_Y);

    #[inline(always)]
    fn mul_by_a(elem: Self::BaseField) -> Self::BaseField {
        <ArkConfig as SWCurveConfig>::mul_by_a(elem)
    }

    // Verbatim copy of upstream implementation.
    // Can't call it directly because of different `Affine` config.
    #[inline]
    fn is_in_correct_subgroup_assuming_on_curve(p: &Affine<Self>) -> bool {
        let x_times_p = p.mul_bigint(crate::Config::<H>::X);
        if x_times_p.eq(p) && !p.infinity {
            return false;
        }

        let minus_x_squared_times_p = x_times_p.mul_bigint(crate::Config::<H>::X).neg();
        let endomorphism_p = endomorphism(p);
        minus_x_squared_times_p.eq(&endomorphism_p)
    }

    // Verbatim copy of upstream implementation.
    // Can't call it directly because of different `Affine` config.
    #[inline]
    fn clear_cofactor(p: &Affine<Self>) -> Affine<Self> {
        let h_eff =
            one_minus_x(crate::Config::<H>::X_IS_NEGATIVE, crate::Config::<H>::X).into_bigint();
        Self::mul_affine(p, h_eff.as_ref()).into()
    }

    // Verbatim copy of upstream implementation.
    // Can't call it directly because of different `Affine` config.
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        compress: Compress,
        validate: Validate,
    ) -> Result<Affine<Self>, SerializationError> {
        let p = if compress == Compress::Yes {
            read_g1_compressed(&mut reader)?
        } else {
            read_g1_uncompressed(&mut reader)?
        };

        if validate == Validate::Yes && !p.is_in_correct_subgroup_assuming_on_curve() {
            return Err(SerializationError::InvalidData);
        }
        Ok(p)
    }

    // Verbatim copy of upstream implementation.
    // Can't call it directly because of different `Affine` config.
    fn serialize_with_mode<W: Write>(
        item: &Affine<Self>,
        mut writer: W,
        compress: Compress,
    ) -> Result<(), SerializationError> {
        let encoding = EncodingFlags {
            is_compressed: compress == Compress::Yes,
            is_infinity: item.is_zero(),
            is_lexographically_largest: item.y > -item.y,
        };
        let mut p = *item;
        if encoding.is_infinity {
            p = Affine::<Self>::zero();
        }
        // need to access the field struct `x` directly, otherwise we get None from xy()
        // method
        let x_bytes = serialize_fq(p.x);
        if encoding.is_compressed {
            let mut bytes: [u8; G1_SERIALIZED_SIZE] = x_bytes;

            encoding.encode_flags(&mut bytes);
            writer.write_all(&bytes)?;
        } else {
            let mut bytes = [0u8; 2 * G1_SERIALIZED_SIZE];
            bytes[0..G1_SERIALIZED_SIZE].copy_from_slice(&x_bytes[..]);
            bytes[G1_SERIALIZED_SIZE..].copy_from_slice(&serialize_fq(p.y)[..]);

            encoding.encode_flags(&mut bytes);
            writer.write_all(&bytes)?;
        };

        Ok(())
    }

    fn serialized_size(compress: Compress) -> usize {
        <ArkConfig as SWCurveConfig>::serialized_size(compress)
    }

    /// Multi scalar multiplication jumping into the user-defined `msm_g1` hook.
    ///
    /// On any internal error returns `Err(0)`.
    fn msm(
        bases: &[Affine<Self>],
        scalars: &[Self::ScalarField],
    ) -> Result<Projective<Self>, usize> {
        let bases: ArkScale<&[Affine<Self>]> = bases.into();
        let scalars: ArkScale<&[Self::ScalarField]> = scalars.into();

        let res = H::bls12_381_msm_g1(bases.encode(), scalars.encode()).unwrap_or_default();

        let res = <ArkScaleProjective<Projective<Self>> as Decode>::decode(&mut res.as_slice());
        res.map_err(|_| 0).map(|res| res.0)
    }

    /// Projective multiplication jumping into the user-defined `mul_projective` hook.
    ///
    /// On any internal error returns `Projective::zero()`.
    fn mul_projective(base: &Projective<Self>, scalar: &[u64]) -> Projective<Self> {
        let base: ArkScaleProjective<Projective<Self>> = (*base).into();
        let scalar: ArkScale<&[u64]> = scalar.into();

        let res =
            H::bls12_381_mul_projective_g1(base.encode(), scalar.encode()).unwrap_or_default();

        let res = ArkScaleProjective::<Projective<Self>>::decode(&mut res.as_slice());
        res.map(|v| v.0).unwrap_or_default()
    }

    /// Affine multiplication jumping into the user-defined `mul_projective` hook.
    ///
    /// On any internal error returns `Projective::zero()`.
    fn mul_affine(base: &Affine<Self>, scalar: &[u64]) -> Projective<Self> {
        Self::mul_projective(&(*base).into(), scalar)
    }
}

fn one_minus_x(
    x_is_negative: bool,
    x_value: &'static [u64],
) -> <ArkConfig as CurveConfig>::ScalarField {
    let x = <ArkConfig as CurveConfig>::ScalarField::from_sign_and_limbs(!x_is_negative, x_value);
    <ArkConfig as CurveConfig>::ScalarField::one() - x
}

pub fn endomorphism<T: CurveHooks>(p: &Affine<Config<T>>) -> Affine<Config<T>> {
    // Endomorphism of the points on the curve.
    // endomorphism_p(x,y) = (BETA * x, y)
    // where BETA is a non-trivial cubic root of unity in fq.
    let mut res = *p;
    res.x *= BETA;
    res
}
