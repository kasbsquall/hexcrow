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

// Status is stored as an enum variant index: Issued 0, Funded 1, Approved 2, Settled 3, Refunded 4.
const SETTLED: &str = "Array([Integer(3), Array([])])";
const REFUNDED: &str = "Array([Integer(4), Array([])])";

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
            sha256(CODE),
            DEADLINE
        ]);
    }

    fn fund(&mut self) {
        let secret = self.client.secret.clone();
        self.call(&secret, "fund", args![1u64, sha256(b"escrow condition root")]);
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
    f.call_fails(&client, "refund", args![1u64], "deadline has not passed yet");
    set_epoch(&mut f.test, DEADLINE);
    f.call(&client, "refund", args![1u64]);
    assert!(f.state().contains(REFUNDED));
}

#[test]
fn stranger_cannot_fund_approve_settle_or_refund() {
    let mut f = setup();
    f.issue();
    let stranger = f.stranger.secret.clone();
    f.call_fails(&stranger, "fund", args![1u64, sha256(b"x")], "only the client can fund");
    f.fund();
    f.call_fails(&stranger, "approve", args![1u64, CODE.to_vec()], "only the client can approve");
    let client = f.client.secret.clone();
    f.call(&client, "approve", args![1u64, CODE.to_vec()]);
    f.call_fails(&stranger, "settle", args![1u64], "only the contractor can settle");
}

#[test]
fn contractor_cannot_approve_own_invoice() {
    let mut f = setup();
    f.issue();
    f.fund();
    let contractor = f.contractor.secret.clone();
    f.call_fails(&contractor, "approve", args![1u64, CODE.to_vec()], "only the client can approve");
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
    set_epoch(&mut f.test, DEADLINE + 5);
    f.call_fails(&client, "refund", args![1u64], "invoice can no longer be refunded");
}

#[test]
fn no_funding_or_approval_after_deadline() {
    let mut f = setup();
    f.issue();
    set_epoch(&mut f.test, DEADLINE);
    let client = f.client.secret.clone();
    f.call_fails(&client, "fund", args![1u64, sha256(b"late")], "invoice deadline has passed");
}

#[test]
fn issue_rejects_past_deadline_and_self_dealing() {
    let mut f = setup();
    let contractor = f.contractor.secret.clone();
    let client_key = f.client.public;
    let contractor_key = f.contractor.public;
    f.call_fails(&contractor, "issue", args![sha256(b"i"), client_key, sha256(CODE), 5u64], "deadline must be in the future");
    f.call_fails(&contractor, "issue", args![sha256(b"i"), contractor_key, sha256(CODE), DEADLINE], "client and contractor keys must differ");
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
