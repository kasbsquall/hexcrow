// SPDX-License-Identifier: MIT
//
// Hexcrow invoice book.
//
// The money in a Hexcrow milestone never touches this component. It sits in a stealth UTXO whose
// spend conditions are enforced natively by the Ootle engine:
//   claim:   approval code + contractor key, before the claim window closes
//   ruling:  mediator key + contractor key, or mediator key + client key, before it closes
//   refund:  client key, once it has closed
// This book is the public audit trail around that escrow: who may act on an invoice (one-time keys
// only), what the escrow output must look like (its condition root), and what happened to it. No
// amount, account or name is stored.

use std::collections::BTreeMap;

use tari_template_lib::{intrinsics::hash_sha256, prelude::*, types::Hash32};

#[template]
mod hexcrow_book {
    use super::*;

    pub struct HexcrowBook {
        invoices: BTreeMap<u64, Invoice>,
        next_id: u64,
    }

    impl HexcrowBook {
        pub fn new() -> Component<Self> {
            Component::new(Self {
                invoices: BTreeMap::new(),
                next_id: 1,
            })
            // Every method authorises its caller against the one-time keys stored on the invoice,
            // so the component itself is open and has no owner who could rewrite the rules.
            .with_access_rules(ComponentAccessRules::allow_all())
            .with_owner_rule(OwnerRule::None)
            .create()
        }

        /// The contractor opens an invoice. The signer becomes the contractor key for this invoice.
        ///
        /// `invoice_hash` is a salted hash of the invoice document, kept off-chain by both parties.
        /// `mediator_key` is the one-time key of the mediator both parties agreed on; the client
        /// accepts it by funding. The client may approve until `deadline_epoch`; the contractor may
        /// then claim, or ask the mediator for a ruling, for `grace_epochs` more.
        pub fn issue(
            &mut self,
            invoice_hash: Hash32,
            client_key: RistrettoPublicKeyBytes,
            mediator_key: RistrettoPublicKeyBytes,
            deadline_epoch: u64,
            grace_epochs: u64,
        ) {
            let contractor_key = CallerContext::transaction_signer_public_key();
            assert!(deadline_epoch > Consensus::current_epoch(), "deadline must be in the future");
            assert!(grace_epochs >= 1, "the claim window needs at least one epoch of grace");
            assert!(client_key != contractor_key, "client and contractor keys must differ");
            assert!(
                mediator_key != client_key && mediator_key != contractor_key,
                "the mediator must hold a third key"
            );
            let claim_until = deadline_epoch.checked_add(grace_epochs).expect("claim window overflows");

            let id = self.next_id;
            self.next_id += 1;
            self.invoices.insert(id, Invoice {
                invoice_hash,
                contractor_key,
                client_key,
                deadline_epoch,
                claim_until,
                approval_lock: None,
                escrow_root: None,
                approval_code: None,
                status: Status::Issued,
                mediator_key,
                ruling_hash: None,
            });
            emit_event("InvoiceIssued", metadata![
                "invoice" => id.to_string(),
                "deadline_epoch" => deadline_epoch.to_string(),
                "claim_until" => claim_until.to_string(),
            ]);
        }

        /// The client records the escrow it has funded. Meant to run in the same transaction that
        /// creates the stealth escrow output, so the event and the output share one transaction id.
        ///
        /// `approval_lock` is SHA-256 of an approval code only the client knows; the escrow's claim
        /// leaf locks to the same hash. `escrow_root` is the condition root of that output. Anyone
        /// holding the invoice terms can rebuild the expected tree and compare roots; the amount
        /// stays hidden.
        pub fn fund(&mut self, invoice: u64, approval_lock: Hash32, escrow_root: Hash32) {
            let inv = self.invoice_mut(invoice);
            inv.require_signer(inv.client_key, "only the client can fund this invoice");
            assert!(inv.status == Status::Issued, "invoice is not open for funding");
            assert!(Consensus::current_epoch() < inv.deadline_epoch, "invoice deadline has passed");
            inv.approval_lock = Some(approval_lock);
            inv.escrow_root = Some(escrow_root);
            inv.status = Status::Funded;
            emit_event("InvoiceFunded", metadata!["invoice" => invoice.to_string()]);
        }

        /// The client approves the delivered milestone by revealing the approval code. The code
        /// alone cannot move the money: the claim path also needs the contractor's one-time key.
        pub fn approve(&mut self, invoice: u64, approval_code: Vec<u8>) {
            let inv = self.invoice_mut(invoice);
            inv.require_signer(inv.client_key, "only the client can approve this invoice");
            assert!(inv.status == Status::Funded, "invoice is not funded");
            assert!(Consensus::current_epoch() < inv.deadline_epoch, "invoice deadline has passed");
            assert!(Some(hash_sha256(&approval_code)) == inv.approval_lock, "approval code does not match");
            inv.approval_code = Some(approval_code);
            inv.status = Status::Approved;
            emit_event("InvoiceApproved", metadata!["invoice" => invoice.to_string()]);
        }

        /// The contractor marks the invoice paid. Meant to run in the transaction that spends the
        /// escrow output through its claim leaf.
        pub fn settle(&mut self, invoice: u64) {
            let inv = self.invoice_mut(invoice);
            inv.require_signer(inv.contractor_key, "only the contractor can settle this invoice");
            assert!(inv.status == Status::Approved, "invoice is not approved");
            assert!(Consensus::current_epoch() < inv.claim_until, "claim window has closed");
            inv.status = Status::Settled;
            emit_event("InvoiceSettled", metadata!["invoice" => invoice.to_string()]);
        }

        /// The mediator rules that the contractor is owed the money, for an invoice the client
        /// funded but never approved. Meant to run in the transaction that spends the escrow through
        /// its ruling leaf for the contractor, which needs the mediator's and the contractor's keys.
        ///
        /// `ruling_hash` is a salted hash of the mediator's written ruling, kept off chain, so the
        /// ruling can later be shown to whoever needs it and checked against the book.
        pub fn rule_for_contractor(&mut self, invoice: u64, ruling_hash: Hash32) {
            let inv = self.invoice_mut(invoice);
            inv.require_signer(inv.mediator_key, "only the mediator can rule on this invoice");
            inv.require_signer(inv.contractor_key, "a ruling for the contractor needs the contractor's key");
            inv.require_open_for_ruling();
            inv.ruling_hash = Some(ruling_hash);
            inv.status = Status::Settled;
            emit_event("InvoiceRuled", metadata!["invoice" => invoice.to_string(), "for" => "contractor"]);
        }

        /// The mediator rules that the money goes back to the client, for an invoice the client
        /// funded but never approved. Meant to run in the transaction that spends the escrow through
        /// its ruling leaf for the client, which needs the mediator's and the client's keys.
        pub fn rule_for_client(&mut self, invoice: u64, ruling_hash: Hash32) {
            let inv = self.invoice_mut(invoice);
            inv.require_signer(inv.mediator_key, "only the mediator can rule on this invoice");
            inv.require_signer(inv.client_key, "a ruling for the client needs the client's key");
            inv.require_open_for_ruling();
            inv.ruling_hash = Some(ruling_hash);
            inv.status = Status::Refunded;
            emit_event("InvoiceRuled", metadata!["invoice" => invoice.to_string(), "for" => "client"]);
        }

        /// The client closes an unapproved invoice once the claim window has closed. Meant to run in
        /// the transaction that spends the escrow output through its refund leaf.
        pub fn refund(&mut self, invoice: u64) {
            let inv = self.invoice_mut(invoice);
            inv.require_signer(inv.client_key, "only the client can refund this invoice");
            assert!(
                inv.status == Status::Issued || inv.status == Status::Funded,
                "invoice can no longer be refunded"
            );
            assert!(Consensus::current_epoch() >= inv.claim_until, "claim window is still open");
            inv.status = Status::Refunded;
            emit_event("InvoiceRefunded", metadata!["invoice" => invoice.to_string()]);
        }

        fn invoice_mut(&mut self, invoice: u64) -> &mut Invoice {
            self.invoices.get_mut(&invoice).expect("unknown invoice")
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Status {
        Issued,
        Funded,
        Approved,
        Settled,
        Refunded,
    }

    // New fields go last, so the positions readers already decode (invoice hash .. status) stay put.
    #[derive(Debug, Clone)]
    pub struct Invoice {
        pub invoice_hash: Hash32,
        pub contractor_key: RistrettoPublicKeyBytes,
        pub client_key: RistrettoPublicKeyBytes,
        pub deadline_epoch: u64,
        pub claim_until: u64,
        pub approval_lock: Option<Hash32>,
        pub escrow_root: Option<Hash32>,
        pub approval_code: Option<Vec<u8>>,
        pub status: Status,
        pub mediator_key: RistrettoPublicKeyBytes,
        pub ruling_hash: Option<Hash32>,
    }

    impl Invoice {
        /// `expected` must have signed this transaction, either as the seal signer or as an
        /// additional authorising signer. The escrow spend is sealed by the invoice's escrow key
        /// (the key that decrypts the escrow output) and co-signed by the acting parties, so both
        /// shapes have to be accepted.
        fn require_signer(&self, expected: RistrettoPublicKeyBytes, msg: &str) {
            if CallerContext::transaction_signer_public_key() == expected {
                return;
            }
            // Aborts the transaction when `expected` is not among the signers.
            let proof = CallerContext::get_signer_proof_for_public_key(expected);
            assert!(proof.resource_address() == PUBLIC_IDENTITY_RESOURCE_ADDRESS, "{}", msg);
            proof.drop();
        }

        /// A ruling settles a dispute about a funded invoice nobody approved, while the escrow's
        /// ruling leaves are still open.
        fn require_open_for_ruling(&self) {
            assert!(self.status == Status::Funded, "only a funded, unapproved invoice can be ruled on");
            assert!(Consensus::current_epoch() < self.claim_until, "claim window has closed");
        }
    }
}
