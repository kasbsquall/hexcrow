// Portions copyright The Tari Project. SPDX-License-Identifier: BSD-3-Clause (the ported `claim_proof_from_l1`).
//! Turns the burn proof file a Minotari L1 wallet writes into the claim Ootle accepts.
//!
//! Ported from `claim_proof_from_l1` in tari-ootle's `applications/tari_ootle_app_utilities/src/burn_claim_proof.rs`
//! (BSD-3-Clause), which is not published as a crate. The one difference: the L1 `OutputFeatures` are decoded here
//! by hand instead of through the pre-release `tari_transaction_components` crate. The decode is strict, and a wrong
//! field could not slip through anyway: Ootle recomputes the L1 output hash from these fields and checks it against
//! the block's output root, so the claim's dry run fails on any mismatch.

use anyhow::{Context, Result, bail, ensure};
use ootle_byte_type::ToByteType;
use ootle_rs::claim_burn::MinotariBurnClaimProof;
use tari_ootle_common_types::engine_types::confidential::{
    BurnOutput, BurnOutputFeatures, BurnOutputInclusionProof, MmrInclusionProof as ClaimMmrInclusionProof,
};
use tari_sidechain::{BurnClaimProof, MmrInclusionProof};
use tari_template_lib_types::Hash32;

/// `OutputType::Burn` in the L1 consensus encoding.
const OUTPUT_TYPE_BURN: u8 = 2;
/// `SideChainFeatureData::ConfidentialOutput` in the L1 consensus encoding.
const SIDECHAIN_CONFIDENTIAL_OUTPUT: u8 = 2;

/// The L1 output features Ootle keeps for a burn.
struct L1BurnFeatures {
    version: u8,
    maturity: u64,
    claim_public_key: [u8; 32],
    range_proof_type: u8,
}

/// Decodes the borsh `OutputFeatures` of an L1 burn output to the default Ootle chain:
/// version u8, output type u8, maturity u64, coinbase extra (u32 length, empty), `Some` sidechain feature holding a
/// `ConfidentialOutput` with a 32-byte claim key (u32 length prefix) and no sidechain id, then the range proof type.
fn decode_features(bytes: &[u8]) -> Result<L1BurnFeatures> {
    ensure!(bytes.len() == 54, "burn output features are {} bytes; a default-chain burn has 54", bytes.len());
    let u32_at = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().expect("4 bytes"));
    ensure!(bytes[1] == OUTPUT_TYPE_BURN, "the output is type {}, not a burn", bytes[1]);
    ensure!(u32_at(10) == 0, "the burn output carries coinbase extra data");
    ensure!(bytes[14] == 1, "the burn output has no sidechain feature, so it names no claimant");
    ensure!(bytes[15] == SIDECHAIN_CONFIDENTIAL_OUTPUT, "the burn's sidechain feature is not a confidential output");
    ensure!(u32_at(16) == 32, "the burn's claim key is not 32 bytes");
    if bytes[52] != 0 {
        bail!("the burn is tagged for another sidechain; only default-chain burns are supported");
    }
    Ok(L1BurnFeatures {
        version: bytes[0],
        maturity: u64::from_le_bytes(bytes[2..10].try_into().expect("8 bytes")),
        claim_public_key: bytes[20..52].try_into().expect("32 bytes"),
        range_proof_type: bytes[53],
    })
}

fn hash32(hash: tari_sidechain::FixedHash) -> Hash32 {
    Hash32::from_array(hash.into())
}

fn mmr_proof(proof: &MmrInclusionProof) -> ClaimMmrInclusionProof {
    ClaimMmrInclusionProof {
        leaf_index: proof.leaf_index,
        mmr_size: proof.mmr_size,
        path: proof.path.iter().copied().map(hash32).collect(),
        peaks: proof.peaks.iter().copied().map(hash32).collect(),
    }
}

fn bounded<const N: usize>(field: &str, bytes: Vec<u8>) -> Result<bounded_vec::BoundedVec<u8, 1, N>> {
    let len = bytes.len();
    bounded_vec::BoundedVec::<u8, 1, N>::from_vec(bytes)
        .map_err(|_| anyhow::anyhow!("burn output {field} length {len} is out of range"))
}

/// Builds the Ootle claim of an L1 burn from the burn claim proof a Minotari wallet produces.
pub fn claim_proof_from_l1(proof: &BurnClaimProof) -> Result<MinotariBurnClaimProof> {
    let p = &proof.output_proof;
    let output = &p.output;
    let features = decode_features(&output.features)?;
    let metadata_signature = borsh::to_vec(&output.metadata_signature).context("encoding the metadata signature")?;
    Ok(MinotariBurnClaimProof {
        commitment: output.commitment.to_byte_type(),
        ownership_proof: proof.ownership_proof.to_byte_type(),
        value: proof.value,
        output: BurnOutput {
            version: output.version,
            features: BurnOutputFeatures {
                version: features.version,
                maturity: features.maturity,
                claim_public_key: features.claim_public_key.into(),
                sidechain_id: None,
                range_proof_type: features.range_proof_type,
            },
            rangeproof_hash: hash32(output.rangeproof_hash),
            script: bounded("script", output.script.clone())?,
            sender_offset_public_key: output.sender_offset_public_key.to_byte_type(),
            metadata_signature: bounded("metadata signature", metadata_signature)?,
            covenant: bounded("covenant", output.covenant.clone())?,
            encrypted_data: bounded("encrypted data", output.encrypted_data.clone())?,
            minimum_value_promise: output.minimum_value_promise,
        },
        inclusion_proof: BurnOutputInclusionProof {
            block_hash: hash32(p.block_hash),
            normal_output_proof: mmr_proof(&p.normal_output_proof),
            normal_output_mr: hash32(p.normal_output_mr),
            block_output_proof: mmr_proof(&p.block_output_proof),
        },
    })
}
