// SPDX-License-Identifier: MIT
//
// The escrow itself: a stealth TARI output with no key path, only a two-leaf condition tree.
//
//   claim  = HashLock(sha256(approval code)) AND BeforeEpoch(deadline) AND signed by contractor key
//   refund = AfterEpoch(deadline) AND signed by client key
//
// The engine checks these natively. The output is encrypted to the invoice's escrow viewing key,
// which both parties hold: it reads the amount, but cannot move it.

use ootle_rs::template_types::{
    Hash32,
    crypto::RistrettoPublicKeyBytes,
    rule,
    stealth::{AtomicCondition, BuiltinPredicate, HashAlg, SpendCondition},
};
use sha2::{Digest, Sha256};

pub fn sha256(data: &[u8]) -> Hash32 {
    let digest: [u8; 32] = Sha256::digest(data).into();
    Hash32::from(digest)
}

pub struct EscrowTerms {
    pub approval_lock: Hash32,
    pub deadline_epoch: u64,
    pub contractor_key: RistrettoPublicKeyBytes,
    pub client_key: RistrettoPublicKeyBytes,
}

impl EscrowTerms {
    pub fn claim_leaf(&self) -> SpendCondition {
        SpendCondition::all([
            AtomicCondition::Builtin(BuiltinPredicate::HashLock {
                hash: self.approval_lock,
                alg: HashAlg::Sha256,
            }),
            AtomicCondition::Builtin(BuiltinPredicate::BeforeEpoch(self.deadline_epoch)),
            AtomicCondition::AccessRule(rule!(public_key(self.contractor_key))),
        ])
    }

    pub fn refund_leaf(&self) -> SpendCondition {
        SpendCondition::all([
            AtomicCondition::Builtin(BuiltinPredicate::AfterEpoch(self.deadline_epoch)),
            AtomicCondition::AccessRule(rule!(public_key(self.client_key))),
        ])
    }

    pub fn conditions(&self) -> Vec<SpendCondition> {
        vec![self.claim_leaf(), self.refund_leaf()]
    }

    pub fn root(&self) -> anyhow::Result<Hash32> {
        ootle_rs::crypto::stealth::validated_condition_root(&self.conditions())
            .map_err(|e| anyhow::anyhow!("condition tree: {e}"))
    }
}
