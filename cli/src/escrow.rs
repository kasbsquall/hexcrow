// SPDX-License-Identifier: MIT
//
// The escrow itself: a stealth TARI output with no key path, only a four-leaf condition tree.
//
//   claim                 = HashLock(sha256(approval code)) AND BeforeEpoch(claim_until) AND contractor key
//   ruling for contractor = BeforeEpoch(claim_until) AND mediator key AND contractor key
//   ruling for client     = BeforeEpoch(claim_until) AND mediator key AND client key
//   refund                = AfterEpoch(claim_until) AND client key
//
// `claim_until` is the approval deadline plus a grace period, so an approval given in the last
// epoch still leaves the contractor time to claim before the refund leaf opens, and a client who
// never answers still leaves the contractor time to ask the mediator. The mediator can pay only a
// party that signs with it, never itself. The engine checks all of this natively. The output is
// encrypted to the invoice's own escrow viewing key, which both parties hold: it reads the amount,
// but cannot move it.

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
    pub claim_until: u64,
    pub contractor_key: RistrettoPublicKeyBytes,
    pub client_key: RistrettoPublicKeyBytes,
    pub mediator_key: RistrettoPublicKeyBytes,
}

/// The four ways out of the escrow.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Leaf {
    Claim,
    RulingForContractor,
    RulingForClient,
    Refund,
}

fn signed_by(key: RistrettoPublicKeyBytes) -> AtomicCondition {
    AtomicCondition::AccessRule(rule!(public_key(key)))
}

impl EscrowTerms {
    pub fn leaf(&self, leaf: Leaf) -> SpendCondition {
        let open = AtomicCondition::Builtin(BuiltinPredicate::BeforeEpoch(self.claim_until));
        match leaf {
            Leaf::Claim => SpendCondition::all([
                AtomicCondition::Builtin(BuiltinPredicate::HashLock {
                    hash: self.approval_lock,
                    alg: HashAlg::Sha256,
                }),
                open,
                signed_by(self.contractor_key),
            ]),
            Leaf::RulingForContractor => SpendCondition::all([open, signed_by(self.mediator_key), signed_by(self.contractor_key)]),
            Leaf::RulingForClient => SpendCondition::all([open, signed_by(self.mediator_key), signed_by(self.client_key)]),
            Leaf::Refund => SpendCondition::all([
                AtomicCondition::Builtin(BuiltinPredicate::AfterEpoch(self.claim_until)),
                signed_by(self.client_key),
            ]),
        }
    }

    pub fn conditions(&self) -> Vec<SpendCondition> {
        [Leaf::Claim, Leaf::RulingForContractor, Leaf::RulingForClient, Leaf::Refund]
            .into_iter()
            .map(|l| self.leaf(l))
            .collect()
    }

    pub fn root(&self) -> anyhow::Result<Hash32> {
        ootle_rs::crypto::stealth::validated_condition_root(&self.conditions())
            .map_err(|e| anyhow::anyhow!("condition tree: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(b: u8) -> RistrettoPublicKeyBytes {
        // Public keys of small scalars: real curve points, distinct per party.
        use tari_crypto::{keys::PublicKey as _, ristretto::{RistrettoPublicKey, RistrettoSecretKey}};
        use tari_utilities::ByteArray as _;
        let mut bytes = [0u8; 32];
        bytes[0] = b;
        let secret = RistrettoSecretKey::from_canonical_bytes(&bytes).unwrap();
        RistrettoPublicKeyBytes::from_bytes(RistrettoPublicKey::from_secret_key(&secret).as_bytes()).unwrap()
    }

    fn terms(mediator: u8) -> EscrowTerms {
        EscrowTerms {
            approval_lock: sha256(b"code"),
            claim_until: 120,
            contractor_key: key(1),
            client_key: key(2),
            mediator_key: key(mediator),
        }
    }

    #[test]
    fn the_four_leaves_form_an_admissible_tree() {
        let t = terms(3);
        assert_eq!(t.conditions().len(), 4);
        let root = t.root().unwrap();
        // Each leaf can be revealed against the committed root.
        for leaf in [Leaf::Claim, Leaf::RulingForContractor, Leaf::RulingForClient, Leaf::Refund] {
            let (_, r) = ootle_rs::crypto::stealth::script_path_witness(&t.conditions(), &t.leaf(leaf)).unwrap();
            assert_eq!(r, root);
        }
    }

    #[test]
    fn another_mediator_gives_another_root() {
        assert_ne!(terms(3).root().unwrap(), terms(4).root().unwrap());
    }
}
