#[cfg(test)]
mod tests;

use crate::CurveHooks;

use ark_algebra_test_templates::*;
use ark_pallas::PallasConfig as ArkConfig;
use ark_models_ext::CurveConfig;

pub struct TestHooks;

type Config = crate::PallasConfig<TestHooks>;
type Affine = crate::Affine<TestHooks>;
type Projective = crate::Projective<TestHooks>;

impl CurveHooks for TestHooks {
    fn pallas_msm(
        bases: &[Affine],
        scalars: &[<Config as CurveConfig>::ScalarField],
    ) -> Result<Projective, ()> {
        test_utils::msm_sw_generic::<Config, ArkConfig>(bases, scalars)
    }

    fn pallas_mul_projective(
        base: &Projective,
        scalar: &[u64],
    ) -> Result<Projective, ()> {
        test_utils::mul_projective_sw_generic::<Config, ArkConfig>(base, scalar)
    }
}

test_group!(sw; SWProjective; sw);