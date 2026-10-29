// SPDX-License-Identifier: MIT
//
// Engine tests for the Hexcrow invoice book. They run the compiled WASM in the same engine the
// network uses (tari_template_test_tooling), so a rejection here is a rejection on Esmeralda.

use sha2::{Digest, Sha256};
use tari_template_test_tooling::{
    TemplateTest,
    byte_type::ToByteType,
    crypto::{RistrettoPublicKey, RistrettoSecretKey},
    engine_types::virtual_substate::{VirtualSubstate, VirtualSubstateId},
    support::assert_error::assert_reject_reason,
    template_lib_types::{ComponentAddress, Hash32, crypto::RistrettoPublicKeyBytes},
    transaction::{args, builder::named_args::NamedArg},
};

const CODE: &[u8] = b"milestone-2 accepted: 31 Oct";
const DEADLINE: u64 = 100;
const GRACE: u64 = 3;
const CLAIM_UNTIL: u64 = DEADLINE + GRACE;

// Status is stored as an enum variant index: Issued 0, Funded 1, Approved 2, Settled 3, Refunded 4.
const SETTLED: &str = "Array([Integer(3), Array([])])";
const REFUNDED: &str = "Array([Integer(4), Array([])])";

// The engine aborts when the book asks for a signer proof of a key that did not sign.
const NOT_A_SIGNER: &str = "out of scope signer badge";

struct Party {
    secret: RistrettoSecretKey,
    public: RistrettoPublicKeyBytes,
}

struct Fixture {
    test: TemplateTest,
    book: ComponentAddress,
    contractor: Party,
    client: Party,
    stranger: Party,
}

fn party(test: &mut TemplateTest, seed: u8) -> Party {
    let (secret, public): (RistrettoSecretKey, RistrettoPublicKey) = test.new_key_pair(seed);
    Party {
        secret,
        public: public.to_byte_type(),
    }
}

fn sha256(data: &[u8]) -> Hash32 {
    let digest: [u8; 32] = Sha256::digest(data).into();
    Hash32::from(digest)
}

fn set_epoch(test: &mut TemplateTest, epoch: u64) {
    test.set_virtual_substate(VirtualSubstateId::CurrentEpoch, VirtualSubstate::CurrentEpoch(epoch));
}

fn setup() -> Fixture {
    let mut test = TemplateTest::my_crate();
    set_epoch(&mut test, 10);
    let template = test.get_template_address("HexcrowBook");
    test.execute_expect_success(
        test.transaction()
            .call_function(template, "new", args![])
            .build_and_seal(test.secret_key()),
        vec![],
    );
    let (book, _) = test
        .read_only_state_store()
        .get_components_by_template_address(template)
        .unwrap()
        .remove(0);
    let contractor = party(&mut test, 11);
    let client = party(&mut test, 12);
    let stranger = party(&mut test, 13);
    Fixture {
        test,
        book,
        contractor,
        client,
        stranger,
    }
}

impl Fixture {
    fn call(&mut self, signer: &RistrettoSecretKey, method: &str, args: Vec<NamedArg>) {
        let tx = self
            .test
            .transaction()
            .call_method(self.book, method, args)
            .build_and_seal(signer);
        self.test.execute_expect_success(tx, vec![]);
    }

    fn call_fails(
        &mut self,
        signer: &RistrettoSecretKey,
        method: &str,
        args: Vec<NamedArg>,
        reason: &str,
    ) {
        let tx = self
            .test
            .transaction()
            .call_method(self.book, method, args)
            .build_and_seal(signer);
        let reject = self.test.execute_expect_failure(tx, vec![]);
        assert_reject_reason(reject, reason);
    }

    fn issue(&mut self) {
        let client_key = self.client.public;
        let secret = self.contractor.secret.clone();
        self.call(&secret, "issue", args![
            sha256(b"invoice #2: brand system, milestone 2"),
            client_key,
            DEADLINE,
            GRACE
        ]);
    }

    fn fund(&mut self) {
        let secret = self.client.secret.clone();
        self.call(&secret, "fund", args![1u64, sha256(CODE), sha256(b"escrow condition root")]);
    }

    fn state(&self) -> String {
        format!("{:?}", self.test.read_only_state_store().get_component(self.book).unwrap().state())
    }
}

#[test]
fn happy_path_issue_fund_approve_settle() {
    let mut f = setup();
    f.issue();
    f.fund();
    let client = f.client.secret.clone();
    f.call(&client, "approve", args![1u64, CODE.to_vec()]);
    let contractor = f.contractor.secret.clone();
    f.call(&contractor, "settle", args![1u64]);
    let state = f.state();
    assert!(state.contains(SETTLED), "invoice 1 should be settled, state: {state}");
}

#[test]
fn refund_after_deadline() {
    let mut f = setup();
    f.issue();
    f.fund();
    let client = f.client.secret.clone();
    f.call_fails(&client, "refund", args![1u64], "claim window is still open");
    set_epoch(&mut f.test, DEADLINE);
    f.call_fails(&client, "refund", args![1u64], "claim window is still open");
    set_epoch(&mut f.test, CLAIM_UNTIL);
    f.call(&client, "refund", args![1u64]);
    assert!(f.state().contains(REFUNDED));
}

#[test]
fn stranger_cannot_fund_approve_settle_or_refund() {
    let mut f = setup();
    f.issue();
    let stranger = f.stranger.secret.clone();
    f.call_fails(&stranger, "fund", args![1u64, sha256(CODE), sha256(b"x")], NOT_A_SIGNER);
    f.fund();
    f.call_fails(&stranger, "approve", args![1u64, CODE.to_vec()], NOT_A_SIGNER);
    let client = f.client.secret.clone();
    f.call(&client, "approve", args![1u64, CODE.to_vec()]);
    f.call_fails(&stranger, "settle", args![1u64], NOT_A_SIGNER);
}

#[test]
fn contractor_cannot_approve_own_invoice() {
    let mut f = setup();
    f.issue();
    f.fund();
    let contractor = f.contractor.secret.clone();
    f.call_fails(&contractor, "approve", args![1u64, CODE.to_vec()], NOT_A_SIGNER);
}

#[test]
fn wrong_approval_code_is_rejected() {
    let mut f = setup();
    f.issue();
    f.fund();
    let client = f.client.secret.clone();
    f.call_fails(&client, "approve", args![1u64, b"guess".to_vec()], "approval code does not match");
}

#[test]
fn settle_requires_approval_and_approve_only_once() {
    let mut f = setup();
    f.issue();
    f.fund();
    let contractor = f.contractor.secret.clone();
    f.call_fails(&contractor, "settle", args![1u64], "invoice is not approved");
    let client = f.client.secret.clone();
    f.call(&client, "approve", args![1u64, CODE.to_vec()]);
    f.call_fails(&client, "approve", args![1u64, CODE.to_vec()], "invoice is not funded");
}

#[test]
fn approved_invoice_cannot_be_refunded() {
    let mut f = setup();
    f.issue();
    f.fund();
    let client = f.client.secret.clone();
    f.call(&client, "approve", args![1u64, CODE.to_vec()]);
    set_epoch(&mut f.test, CLAIM_UNTIL + 5);
    f.call_fails(&client, "refund", args![1u64], "invoice can no longer be refunded");
}

#[test]
fn no_funding_or_approval_after_deadline() {
    let mut f = setup();
    f.issue();
    set_epoch(&mut f.test, DEADLINE);
    let client = f.client.secret.clone();
    f.call_fails(&client, "fund", args![1u64, sha256(CODE), sha256(b"late")], "invoice deadline has passed");
}

#[test]
fn issue_rejects_past_deadline_and_self_dealing() {
    let mut f = setup();
    let contractor = f.contractor.secret.clone();
    let client_key = f.client.public;
    let contractor_key = f.contractor.public;
    f.call_fails(&contractor, "issue", args![sha256(b"i"), client_key, 5u64, GRACE], "deadline must be in the future");
    f.call_fails(&contractor, "issue", args![sha256(b"i"), client_key, DEADLINE, 0u64], "the claim window needs at least one epoch of grace");
    f.call_fails(&contractor, "issue", args![sha256(b"i"), contractor_key, DEADLINE, GRACE], "client and contractor keys must differ");
}

#[test]
fn state_holds_no_amount_and_no_account() {
    let mut f = setup();
    f.issue();
    f.fund();
    let state = f.state();
    // The book only knows hashes, one-time keys, the deadline and the status.
    assert!(!state.to_lowercase().contains("amount"));
    assert!(!state.contains("account"));
}

#[test]
fn escrow_key_seals_and_contractor_cosigns_settle() {
    // On Esmeralda the escrow spend is sealed by the invoice's escrow key (it decrypts the escrow
    // output) and co-signed by the contractor. The book must accept the co-signer.
    let mut f = setup();
    f.issue();
    f.fund();
    let client = f.client.secret.clone();
    f.call(&client, "approve", args![1u64, CODE.to_vec()]);
    let escrow = party(&mut f.test, 14);
    let tx = f
        .test
        .transaction()
        .call_method(f.book, "settle", args![1u64])
        .add_signer(&escrow.public, &f.contractor.secret)
        .seal(&escrow.secret);
    f.test.execute_expect_success(tx, vec![]);
    assert!(f.state().contains(SETTLED));
}

#[test]
fn escrow_key_alone_cannot_settle() {
    let mut f = setup();
    f.issue();
    f.fund();
    let client = f.client.secret.clone();
    f.call(&client, "approve", args![1u64, CODE.to_vec()]);
    let escrow = party(&mut f.test, 14);
    let tx = f
        .test
        .transaction()
        .call_method(f.book, "settle", args![1u64])
        .build_and_seal(&escrow.secret);
    let reject = f.test.execute_expect_failure(tx, vec![]);
    assert_reject_reason(reject, NOT_A_SIGNER);
}

#[test]
fn only_the_client_sets_the_approval_lock() {
    // The contractor issues without a lock, so it cannot pick a code it already knows. The lock
    // arrives with the client's funding, and only the client can fund.
    let mut f = setup();
    f.issue();
    let contractor = f.contractor.secret.clone();
    f.call_fails(&contractor, "fund", args![1u64, sha256(b"contractor's own code"), sha256(b"r")], NOT_A_SIGNER);
    f.fund();
    let client = f.client.secret.clone();
    f.call_fails(&client, "approve", args![1u64, b"contractor's own code".to_vec()], "approval code does not match");
    f.call(&client, "approve", args![1u64, CODE.to_vec()]);
}

#[test]
fn approval_in_the_last_epoch_still_leaves_the_claim_window() {
    let mut f = setup();
    f.issue();
    f.fund();
    set_epoch(&mut f.test, DEADLINE - 1);
    let client = f.client.secret.clone();
    f.call(&client, "approve", args![1u64, CODE.to_vec()]);
    set_epoch(&mut f.test, CLAIM_UNTIL - 1);
    f.call_fails(&client, "refund", args![1u64], "invoice can no longer be refunded");
    let contractor = f.contractor.secret.clone();
    f.call(&contractor, "settle", args![1u64]);
    assert!(f.state().contains(SETTLED));
}

#[test]
fn no_approval_after_deadline_and_no_settle_after_claim_window() {
    let mut f = setup();
    f.issue();
    f.fund();
    let client = f.client.secret.clone();
    set_epoch(&mut f.test, DEADLINE);
    f.call_fails(&client, "approve", args![1u64, CODE.to_vec()], "invoice deadline has passed");
    set_epoch(&mut f.test, DEADLINE - 1);
    f.call(&client, "approve", args![1u64, CODE.to_vec()]);
    set_epoch(&mut f.test, CLAIM_UNTIL);
    let contractor = f.contractor.secret.clone();
    f.call_fails(&contractor, "settle", args![1u64], "claim window has closed");
}
