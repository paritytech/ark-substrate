#![cfg_attr(not(feature = "std"), no_std)]

use ark_ec::{
    pairing::{MillerLoopOutput, Pairing, PairingOutput},
    short_weierstrass,
    short_weierstrass::SWCurveConfig,
    twisted_edwards,
    twisted_edwards::TECurveConfig,
    CurveConfig, VariableBaseMSM,
};
use ark_scale::{hazmat::ArkScaleProjective, ArkScale};
use ark_std::vec::Vec;
use codec::{Decode, Encode};

pub fn multi_miller_loop_generic<Curve: Pairing>(g1: Vec<u8>, g2: Vec<u8>) -> Result<Vec<u8>, ()> {
    let g1 = <ArkScale<Vec<<Curve as Pairing>::G1Affine>> as Decode>::decode(&mut g1.as_slice())
        .map_err(|_| ())?;
    let g2 = <ArkScale<Vec<<Curve as Pairing>::G2Affine>> as Decode>::decode(&mut g2.as_slice())
        .map_err(|_| ())?;

    let result = Curve::multi_miller_loop(g1.0, g2.0).0;

    let result: ArkScale<<Curve as Pairing>::TargetField> = result.into();
    Ok(result.encode())
}

pub fn final_exponentiation_generic<Curve: Pairing>(target: Vec<u8>) -> Result<Vec<u8>, ()> {
    let target =
        <ArkScale<<Curve as Pairing>::TargetField> as Decode>::decode(&mut target.as_slice())
            .map_err(|_| ())?;

    let result = Curve::final_exponentiation(MillerLoopOutput(target.0)).ok_or(())?;

    let result: ArkScale<PairingOutput<Curve>> = result.into();
    Ok(result.encode())
}

pub fn msm_sw_generic<Curve: SWCurveConfig>(
    bases: Vec<u8>,
    scalars: Vec<u8>,
) -> Result<Vec<u8>, ()> {
    let bases =
        <ArkScale<Vec<short_weierstrass::Affine<Curve>>> as Decode>::decode(&mut bases.as_slice())
            .map_err(|_| ())?;
    let scalars = <ArkScale<Vec<<Curve as CurveConfig>::ScalarField>> as Decode>::decode(
        &mut scalars.as_slice(),
    )
    .map_err(|_| ())?;

    let result =
        <short_weierstrass::Projective<Curve> as VariableBaseMSM>::msm(&bases.0, &scalars.0)
            .map_err(|_| ())?;

    let result: ArkScaleProjective<short_weierstrass::Projective<Curve>> = result.into();
    Ok(result.encode())
}

pub fn msm_te_generic<Curve: TECurveConfig>(
    bases: Vec<u8>,
    scalars: Vec<u8>,
) -> Result<Vec<u8>, ()> {
    let bases =
        <ArkScale<Vec<twisted_edwards::Affine<Curve>>> as Decode>::decode(&mut bases.as_slice())
            .map_err(|_| ())?;
    let scalars = <ArkScale<Vec<<Curve as CurveConfig>::ScalarField>> as Decode>::decode(
        &mut scalars.as_slice(),
    )
    .map_err(|_| ())?;

    let result = <twisted_edwards::Projective<Curve> as VariableBaseMSM>::msm(&bases.0, &scalars.0)
        .map_err(|_| ())?;

    let result: ArkScaleProjective<twisted_edwards::Projective<Curve>> = result.into();
    Ok(result.encode())
}

pub fn mul_projective_generic<Group: SWCurveConfig>(
    base: Vec<u8>,
    scalar: Vec<u8>,
) -> Result<Vec<u8>, ()> {
    let base = <ArkScaleProjective<short_weierstrass::Projective<Group>> as Decode>::decode(
        &mut base.as_slice(),
    )
    .map_err(|_| ())?;
    let scalar = <ArkScale<Vec<u64>> as Decode>::decode(&mut scalar.as_slice()).map_err(|_| ())?;

    let result = <Group as SWCurveConfig>::mul_projective(&base.0, &scalar.0);

    let result: ArkScaleProjective<short_weierstrass::Projective<Group>> = result.into();
    Ok(result.encode())
}

pub fn mul_projective_te_generic<Group: TECurveConfig>(
    base: Vec<u8>,
    scalar: Vec<u8>,
) -> Result<Vec<u8>, ()> {
    let base = <ArkScaleProjective<twisted_edwards::Projective<Group>> as Decode>::decode(
        &mut base.as_slice(),
    )
    .map_err(|_| ())?;
    let scalar = <ArkScale<Vec<u64>> as Decode>::decode(&mut scalar.as_slice()).map_err(|_| ())?;

    let result = <Group as TECurveConfig>::mul_projective(&base.0, &scalar.0);

    let result: ArkScaleProjective<twisted_edwards::Projective<Group>> = result.into();
    Ok(result.encode())
}