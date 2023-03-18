#![cfg_attr(not(feature = "std"), no_std)]
use crate::HostFunctions;
use ark_algebra_test_templates::*;
use ark_std::vec::Vec;

pub struct Host {}

impl HostFunctions for Host {
    fn ed_on_bls12_377_msm(bases: Vec<u8>, scalars: Vec<u8>) -> Vec<u8> {
        sp_io::elliptic_curves::ed_on_bls12_377_msm(bases, scalars)
    }
}

test_group!(te; crate::EdwardsProjective<super::Host>; te);
