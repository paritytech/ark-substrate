use ark_bls12_381::{fq2::Fq2, fr::Fr, Fq};
use ark_ff::{Field, MontFp, Zero};
use ark_scale::{
    ark_serialize::{Compress, SerializationError, Validate},
    hazmat::ArkScaleProjective,
    scale::{Decode, Encode},
};
use ark_std::{
    io::{Read, Write},
    marker::PhantomData,
    ops::Neg,
};
use sp_ark_models::{
    bls12,
    bls12::Bls12Config,
    short_weierstrass::{Affine, Projective, SWCurveConfig},
    AffineRepr, CurveConfig, CurveGroup, Group,
};

use crate::{
    g1,
    util::{
        read_g2_compressed, read_g2_uncompressed, serialize_fq, EncodingFlags, G2_SERIALIZED_SIZE,
    },
    ArkScale, HostFunctions,
};

pub type G2Affine<H> = bls12::G2Affine<crate::Config<H>>;
pub type G2Projective<H> = bls12::G2Projective<crate::Config<H>>;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Config<H: HostFunctions>(PhantomData<fn() -> H>);

impl<H: HostFunctions> CurveConfig for Config<H> {
    type BaseField = Fq2;
    type ScalarField = Fr;

    /// COFACTOR = (x^8 - 4 x^7 + 5 x^6) - (4 x^4 + 6 x^3 - 4 x^2 - 4 x + 13) //
    /// 9
    /// = 305502333931268344200999753193121504214466019254188142667664032982267604182971884026507427359259977847832272839041616661285803823378372096355777062779109
    #[rustfmt::skip]
    const COFACTOR: &'static [u64] = &[
        0xcf1c38e31c7238e5,
        0x1616ec6e786f0c70,
        0x21537e293a6691ae,
        0xa628f1cb4d9e82ef,
        0xa68a205b2e5a7ddf,
        0xcd91de4547085aba,
        0x91d50792876a202,
        0x5d543a95414e7f1,
    ];

    /// COFACTOR_INV = COFACTOR^{-1} mod r
    /// 26652489039290660355457965112010883481355318854675681319708643586776743290055
    const COFACTOR_INV: Fr =
        MontFp!("26652489039290660355457965112010883481355318854675681319708643586776743290055");
}

impl<H: HostFunctions> SWCurveConfig for Config<H> {
    /// COEFF_A = [0, 0]
    const COEFF_A: Fq2 = Fq2::new(g1::Config::<H>::COEFF_A, g1::Config::<H>::COEFF_A);

    /// COEFF_B = [4, 4]
    const COEFF_B: Fq2 = Fq2::new(g1::Config::<H>::COEFF_B, g1::Config::<H>::COEFF_B);

    /// AFFINE_GENERATOR_COEFFS = (G2_GENERATOR_X, G2_GENERATOR_Y)
    const GENERATOR: Affine<Self> = Affine::<Self>::new_unchecked(G2_GENERATOR_X, G2_GENERATOR_Y);

    #[inline(always)]
    fn mul_by_a(_: Self::BaseField) -> Self::BaseField {
        Self::BaseField::zero()
    }

    fn is_in_correct_subgroup_assuming_on_curve(point: &Affine<Self>) -> bool {
        // Algorithm From Section 4 of https://eprint.iacr.org/2021/1130.
        //
        // Checks that [p]P = [X]P

        let mut x_times_point = point.mul_bigint(crate::Config::<H>::X);
        if crate::Config::<H>::X_IS_NEGATIVE {
            x_times_point = -x_times_point;
        }

        let p_times_point = p_power_endomorphism(point);

        x_times_point.eq(&p_times_point)
    }

    #[inline]
    fn clear_cofactor(p: &Affine<Self>) -> Affine<Self> {
        // Based on Section 4.1 of https://eprint.iacr.org/2017/419.pdf
        // [h(ψ)]P = [x^2 − x − 1]P + [x − 1]ψ(P) + (ψ^2)(2P)

        // x = -15132376222941642752
        // When multiplying, use -c1 instead, and then negate the result. That's much
        // more efficient, since the scalar -c1 has less limbs and a much lower Hamming
        // weight.
        let x: &'static [u64] = crate::Config::<H>::X;
        let p_projective = p.into_group();

        // [x]P
        let x_p = Config::mul_affine(p, x).neg();
        // ψ(P)
        let psi_p = p_power_endomorphism(p);
        // (ψ^2)(2P)
        let mut psi2_p2 = double_p_power_endomorphism(&p_projective.double());

        // tmp = [x]P + ψ(P)
        let mut tmp = x_p;
        tmp += &psi_p;

        // tmp2 = [x^2]P + [x]ψ(P)
        let mut tmp2: Projective<Config<H>> = tmp;
        tmp2 = tmp2.mul_bigint(x).neg();

        // add up all the terms
        psi2_p2 += tmp2;
        psi2_p2 -= x_p;
        psi2_p2 += &-psi_p;
        (psi2_p2 - p_projective).into_affine()
    }

    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        compress: Compress,
        validate: Validate,
    ) -> Result<Affine<Self>, SerializationError> {
        let p = if compress == Compress::Yes {
            read_g2_compressed(&mut reader)?
        } else {
            read_g2_uncompressed(&mut reader)?
        };

        if validate == Validate::Yes && !p.is_in_correct_subgroup_assuming_on_curve() {
            return Err(SerializationError::InvalidData);
        }
        Ok(p)
    }

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

        let mut x_bytes = [0u8; G2_SERIALIZED_SIZE];
        let c1_bytes = serialize_fq(p.x.c1);
        let c0_bytes = serialize_fq(p.x.c0);
        x_bytes[0..48].copy_from_slice(&c1_bytes[..]);
        x_bytes[48..96].copy_from_slice(&c0_bytes[..]);
        if encoding.is_compressed {
            let mut bytes: [u8; G2_SERIALIZED_SIZE] = x_bytes;

            encoding.encode_flags(&mut bytes);
            writer.write_all(&bytes)?;
        } else {
            let mut bytes = [0u8; 2 * G2_SERIALIZED_SIZE];

            let mut y_bytes = [0u8; G2_SERIALIZED_SIZE];
            let c1_bytes = serialize_fq(p.y.c1);
            let c0_bytes = serialize_fq(p.y.c0);
            y_bytes[0..48].copy_from_slice(&c1_bytes[..]);
            y_bytes[48..96].copy_from_slice(&c0_bytes[..]);
            bytes[0..G2_SERIALIZED_SIZE].copy_from_slice(&x_bytes);
            bytes[G2_SERIALIZED_SIZE..].copy_from_slice(&y_bytes);

            encoding.encode_flags(&mut bytes);
            writer.write_all(&bytes)?;
        };

        Ok(())
    }

    fn serialized_size(compress: Compress) -> usize {
        if compress == Compress::Yes {
            G2_SERIALIZED_SIZE
        } else {
            2 * G2_SERIALIZED_SIZE
        }
    }

    fn msm(
        bases: &[Affine<Self>],
        scalars: &[<Self as CurveConfig>::ScalarField],
    ) -> Result<Projective<Self>, usize> {
        let bases: ArkScale<&[Affine<Self>]> = bases.into();
        let scalars: ArkScale<&[<Self as CurveConfig>::ScalarField]> = scalars.into();

        let result = H::bls12_381_msm_g2(bases.encode(), scalars.encode()).unwrap();

        let result =
            <ArkScaleProjective<Projective<Self>> as Decode>::decode(&mut result.as_slice());
        result.map_err(|_| 0).map(|res| res.0)
    }

    fn mul_projective(base: &Projective<Self>, scalar: &[u64]) -> Projective<Self> {
        let base: ArkScaleProjective<Projective<Self>> = (*base).into();
        let scalar: ArkScale<&[u64]> = scalar.into();

        let result = H::bls12_381_mul_projective_g2(base.encode(), scalar.encode()).unwrap();

        let result =
            <ArkScaleProjective<Projective<Self>> as Decode>::decode(&mut result.as_slice());
        result.unwrap().0
    }

    fn mul_affine(base: &Affine<Self>, scalar: &[u64]) -> Projective<Self> {
        let base: Projective<Self> = (*base).into();
        let base: ArkScaleProjective<Projective<Self>> = base.into();
        let scalar: ArkScale<&[u64]> = scalar.into();

        let result = H::bls12_381_mul_projective_g2(base.encode(), scalar.encode()).unwrap();

        let result =
            <ArkScaleProjective<Projective<Self>> as Decode>::decode(&mut result.as_slice());
        result.unwrap().0
    }
}

pub const G2_GENERATOR_X: Fq2 = Fq2::new(G2_GENERATOR_X_C0, G2_GENERATOR_X_C1);
pub const G2_GENERATOR_Y: Fq2 = Fq2::new(G2_GENERATOR_Y_C0, G2_GENERATOR_Y_C1);

/// G2_GENERATOR_X_C0 =
/// 352701069587466618187139116011060144890029952792775240219908644239793785735715026873347600343865175952761926303160
pub const G2_GENERATOR_X_C0: Fq = MontFp!("352701069587466618187139116011060144890029952792775240219908644239793785735715026873347600343865175952761926303160");

/// G2_GENERATOR_X_C1 =
/// 3059144344244213709971259814753781636986470325476647558659373206291635324768958432433509563104347017837885763365758
pub const G2_GENERATOR_X_C1: Fq = MontFp!("3059144344244213709971259814753781636986470325476647558659373206291635324768958432433509563104347017837885763365758");

/// G2_GENERATOR_Y_C0 =
/// 1985150602287291935568054521177171638300868978215655730859378665066344726373823718423869104263333984641494340347905
pub const G2_GENERATOR_Y_C0: Fq = MontFp!("1985150602287291935568054521177171638300868978215655730859378665066344726373823718423869104263333984641494340347905");

/// G2_GENERATOR_Y_C1 =
/// 927553665492332455747201965776037880757740193453592970025027978793976877002675564980949289727957565575433344219582
pub const G2_GENERATOR_Y_C1: Fq = MontFp!("927553665492332455747201965776037880757740193453592970025027978793976877002675564980949289727957565575433344219582");

// PSI_X = 1/(u+1)^((p-1)/3)
const P_POWER_ENDOMORPHISM_COEFF_0 : Fq2 = Fq2::new(
    Fq::ZERO,
    MontFp!(
                "4002409555221667392624310435006688643935503118305586438271171395842971157480381377015405980053539358417135540939437"
    )
);

// PSI_Y = 1/(u+1)^((p-1)/2)
const P_POWER_ENDOMORPHISM_COEFF_1: Fq2 = Fq2::new(
    MontFp!(
                "2973677408986561043442465346520108879172042883009249989176415018091420807192182638567116318576472649347015917690530"),
    MontFp!(
       "1028732146235106349975324479215795277384839936929757896155643118032610843298655225875571310552543014690878354869257")
);

// PSI_2_X = (u+1)^((1-p^2)/3)
const DOUBLE_P_POWER_ENDOMORPHISM_COEFF_0: Fq2 = Fq2::new(
    MontFp!("4002409555221667392624310435006688643935503118305586438271171395842971157480381377015405980053539358417135540939436"),
    Fq::ZERO
);

/// psi(P) is the untwist-Frobenius-twist endomorhism on E'(Fq2)
fn p_power_endomorphism<H: HostFunctions>(p: &Affine<Config<H>>) -> Affine<Config<H>> {
    // The p-power endomorphism for G2 is defined as follows:
    // 1. Note that G2 is defined on curve E': y^2 = x^3 + 4(u+1).
    //    To map a point (x, y) in E' to (s, t) in E,
    //    set s = x / ((u+1) ^ (1/3)), t = y / ((u+1) ^ (1/2)),
    //    because E: y^2 = x^3 + 4.
    // 2. Apply theFrobenius endomorphism (s, t) => (s', t'),
    //    another point on curve E, where s' = s^p, t' = t^p.
    // 3. Map the point From E back to E'; that is,
    //    set x' = s' * ((u+1) ^ (1/3)), y' = t' * ((u+1) ^ (1/2)).
    //
    // To sum up, it maps
    // (x,y) -> (x^p / ((u+1)^((p-1)/3)), y^p / ((u+1)^((p-1)/2)))
    // as implemented in the code as follows.

    let mut res = *p;
    res.x.frobenius_map_in_place(1);
    res.y.frobenius_map_in_place(1);

    let tmp_x = res.x;
    res.x.c0 = -P_POWER_ENDOMORPHISM_COEFF_0.c1 * tmp_x.c1;
    res.x.c1 = P_POWER_ENDOMORPHISM_COEFF_0.c1 * tmp_x.c0;
    res.y *= P_POWER_ENDOMORPHISM_COEFF_1;

    res
}

/// For a p-power endomorphism psi(P), compute psi(psi(P))
fn double_p_power_endomorphism<H: HostFunctions>(
    p: &Projective<Config<H>>,
) -> Projective<Config<H>> {
    let mut res = *p;

    res.x *= DOUBLE_P_POWER_ENDOMORPHISM_COEFF_0;
    res.y = res.y.neg();

    res
}
