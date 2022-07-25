// Using https://github.com/near-examples/docs-examples/blob/4fda29c8cdabd9aba90787c553413db7725d88bd/donation-rs/contract/src/lib.rs as a basis

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap};
use near_sdk::{
    env, log, near_bindgen, serde_json, AccountId, Balance, BorshStorageKey, CryptoHash, Gas,
    PanicOnDefault, Promise,
};
use witgen::witgen;

#[witgen]
type Amount = Balance;
type MatcherAccountId = AccountId;
type MatcherAmountMap = UnorderedMap<MatcherAccountId, Amount>; // https://doc.rust-lang.org/reference/items/type-aliases.html
type RecipientAccountId = AccountId;
type MatcherAmountPerRecipient = LookupMap<RecipientAccountId, MatcherAmountMap>;

pub const STORAGE_COST: Amount = 1_000_000_000_000_000_000_000; // ONEDAY: Write this in a more human-readable way, and document how this value was decided.
pub const GAS_FOR_ACCOUNT_CALLBACK: Gas = Gas(20_000_000_000_000); // gas for cross-contract calls, ~5 Tgas (teragas = 1e12) per "hop"

// TODO: Move helper functions to a separate file

/// Used to generate a unique prefix in our storage collections (this is to avoid data collisions; see https://stackoverflow.com/questions/65248816/why-should-i-hash-keys-in-the-nearprotocol-unorderedmap)
pub(crate) fn hash_account_id(account_id: &String) -> CryptoHash {
    env::sha256_array(account_id.as_bytes())
}

/// Helper function to convert yoctoNEAR to $NEAR with 4 decimals of precision.
pub(crate) fn yocto_to_near(yocto: u128) -> f64 {
    //10^20 yoctoNEAR (1 NEAR would be 10_000). This is to give a precision of 4 decimal places.
    let formatted_near = yocto / 100_000_000_000_000_000_000;
    let near = formatted_near as f64 / 10_000_f64;

    near
}

/// Helper function to convert yoctoNEAR to $NEAR with 4 decimals of precision.
pub(crate) fn yocto_to_near_string(yocto: u128) -> String {
    let numeric = yocto_to_near(yocto);
    numeric.to_string() + &" Ⓝ".to_string()
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Recipients,
    RecipientsInner { hash: CryptoHash },
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub recipients: MatcherAmountPerRecipient, // https://docs.near.org/concepts/storage/data-storage#unorderedmap The outer key-value pair is the "recipient: matcher-amount-map". The inner map (matcher amount) has a key-value pair of "matcher: amount".
}

// TODO: Review each part of this repo to ensure that it can scale to large amounts of data.

#[near_bindgen]
impl Contract {
    #[init]
    #[private] // Public - but only callable by env::current_account_id()
    pub fn new() -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            recipients: MatcherAmountPerRecipient::new(StorageKey::Recipients),
        }
    }

    fn create_new_matcher_amount_map(recipient: &AccountId) -> MatcherAmountMap {
        MatcherAmountMap::new(StorageKey::RecipientsInner {
            hash: hash_account_id(&recipient.to_string()),
        })
    }

    fn get_expected_matchers_for_this_recipient(&self, recipient: &AccountId) -> MatcherAmountMap {
        let msg = format!("Could not find any matchers for recipient `{}`", &recipient);
        self.recipients.get(&recipient).expect(&msg)
    }

    fn get_expected_commitment(
        &self,
        recipient: &RecipientAccountId,
        matchers_for_this_recipient: &MatcherAmountMap,
        matcher: &AccountId,
    ) -> Amount {
        let existing_commitment = matchers_for_this_recipient.get(&matcher).expect(
            format!(
                "{} does not currently have any funds committed to {}.",
                matcher, recipient
            )
            .as_str(),
        );
        near_sdk::log!(
            "existing_commitment = {}",
            yocto_to_near_string(existing_commitment)
        );
        existing_commitment
    }

    #[payable] // Public - People can attach money
    pub fn offer_matching_funds(&mut self, recipient: AccountId) -> String {
        let donation_amount: Amount = env::attached_deposit();
        assert!(
            donation_amount > STORAGE_COST,
            "Attach at least {} yoctoNEAR",
            STORAGE_COST
        );
        let matcher = env::signer_account_id(); // https://docs.near.org/develop/contracts/environment/

        // Get the current map for the recipient. If it doesn't exist, create one.
        let mut matchers_for_this_recipient = self
            .recipients
            .get(&recipient)
            .unwrap_or(Self::create_new_matcher_amount_map(&recipient));

        // If the matcher has already donated, increment their donation.
        let existing_commitment = matchers_for_this_recipient.get(&matcher).unwrap_or(0);
        near_sdk::log!(
            "existing_commitment {}",
            yocto_to_near_string(existing_commitment)
        );
        let updated_commitment = donation_amount + existing_commitment;
        near_sdk::log!(
            "updated_commitment {}",
            yocto_to_near_string(updated_commitment)
        );
        matchers_for_this_recipient.insert(&matcher, &updated_commitment);

        self.recipients
            .insert(&recipient, &matchers_for_this_recipient);

        let result = format!(
            "{} is now committed to match donations to {} up to a maximum of {}.",
            matcher,
            recipient,
            yocto_to_near_string(donation_amount)
        );
        log!(result);
        result
    }

    pub fn get_commitments(&self, recipient: AccountId) -> String {
        let mut matchers_log: Vec<String> = Vec::new();
        let matchers_for_this_recipient: MatcherAmountMap =
            self.get_expected_matchers_for_this_recipient(&recipient);
        let matchers = matchers_for_this_recipient.keys_as_vector();
        for (_, matcher) in matchers.iter().enumerate() {
            let existing_commitment = matchers_for_this_recipient.get(&matcher).unwrap();
            let msg = format!(
                "{}: {},",
                matcher,
                yocto_to_near_string(existing_commitment)
            );
            log!(msg);
            matchers_log.push(msg);
        }
        format!("These matchers are committed to match donations to {} up to a maximum of the following amounts:\n{}",recipient,matchers_log.join("\n"))
    }

    pub fn transfer_from_escrow(&self, destination_account: &AccountId, amount: Amount) -> Promise {
        // TODO: Consider subtracting storage cost like https://github.com/near-examples/docs-examples/blob/4fda29c8cdabd9aba90787c553413db7725d88bd/donation-rs/contract/src/lib.rs#L51
        log!(
            "transfer_from_escrow destination_account: {}, amount: {}",
            destination_account,
            yocto_to_near_string(amount)
        );
        Promise::new(destination_account.clone()).transfer(amount)
    }

    /**
     * Gets called via `rescind_matching_funds` and `send_matching_donation`.
     */
    pub fn set_matcher_amount(
        &mut self,
        recipient: &AccountId,
        matcher: &AccountId,
        amount: Amount,
    ) -> MatcherAmountMap {
        near_sdk::log!(
            "set_matcher_amount(recipient: {}, matcher: {}, amount: {})",
            &recipient,
            &matcher,
            &yocto_to_near_string(amount)
        );
        // TODO assert_self();
        // TODO assert_single_promise_success();
        let mut matchers_for_this_recipient =
            self.get_expected_matchers_for_this_recipient(&recipient);
        if amount > 0 {
            let existing_commitment =
                self.get_expected_commitment(&recipient, &matchers_for_this_recipient, &matcher); // TODO Assert that there is a matcher?
            matchers_for_this_recipient.insert(&matcher, &amount);
        } else {
            self.recipients.remove(&matcher);
        }

        matchers_for_this_recipient
    }

    pub fn rescind_matching_funds(
        &mut self,
        recipient: &AccountId,
        requested_withdrawal_amount: Amount,
    ) -> String {
        let matcher = env::signer_account_id();
        let matchers_for_this_recipient = self.get_expected_matchers_for_this_recipient(&recipient);
        let result;
        let amount_already_committed =
            self.get_expected_commitment(&recipient, &matchers_for_this_recipient, &matcher);
        let mut amount_to_decrease = requested_withdrawal_amount;
        let mut new_amount = 0;
        if requested_withdrawal_amount > amount_already_committed {
            amount_to_decrease = amount_already_committed;
            result =
                format!(
                "{} is about to rescind {} and then will not be matching donations to {} anymore",
                matcher, yocto_to_near_string(amount_to_decrease), recipient
            );
        } else {
            new_amount = amount_already_committed - amount_to_decrease;
            result = format!("{} is about to rescind {} and then will only be committed to match donations to {} up to a maximum of {}.", matcher, yocto_to_near_string(amount_to_decrease), recipient, yocto_to_near_string(new_amount));
        }
        self.transfer_from_escrow(&matcher, amount_to_decrease) // Funds go from escrow back to the matcher.
            .then(
                Self::ext(env::current_account_id()) // escrow contract name
                    .with_static_gas(GAS_FOR_ACCOUNT_CALLBACK)
                    .set_matcher_amount(&recipient, &matcher, new_amount),
            );
        result
    }

    fn send_matching_donation(
        &self,
        recipient: &AccountId,
        matchers_for_this_recipient: &MatcherAmountMap,
        matcher: AccountId,
        amount: Amount,
    ) {
        let escrow_contract_name = env::current_account_id(); // https://docs.near.org/develop/contracts/environment/
        let existing_commitment =
            self.get_expected_commitment(&recipient, &matchers_for_this_recipient, &matcher);
        // TODO  const matched_amount: u128 = min(amount, existing_commitment);
        //   const remaining_commitment: u128 = u128.sub(existing_commitment, matched_amount);
        //   logging.log(`${matcher} will send a matching donation of yocto_to_near_string(${matched_amount}) to ${recipient}. Remaining commitment: yocto_to_near_string(${remaining_commitment}).`);
        //   _transfer_from_escrow(recipient, matched_amount)
        //     .then(escrowContractName)
        //     .function_call<RecipientMatcherAmount>('set_matcher_amount', { recipient, matcher, amount: remainingCommitment }, u128.Zero, XCC_GAS);
    }

    fn send_matching_donations(&self, recipient: &AccountId, amount: Amount) {
        let matchers_for_this_recipient = self.get_expected_matchers_for_this_recipient(&recipient);
        let matchers = matchers_for_this_recipient.keys_as_vector();
        for (_, matcher) in matchers.iter().enumerate() {
            self.send_matching_donation(recipient, &matchers_for_this_recipient, matcher, amount);
        }
    }

    pub fn transfer_from_escrow_callback_after_donating(
        donor: AccountId,
        recipient: AccountId,
        amount: Amount,
    ) {
        // TODO  assert_self();
        //   assert_single_promise_success();

        //   logging.log(`transfer_from_escrow_callback_after_donating. ${donor} donated yocto_to_near_string(${amount}) to ${recipient}.`);
        //   _sendMatchingDonations(recipient, amount, escrowContractName);
    }

    pub fn donate(recipient: AccountId) {
        // TODO  const amount = Context.attachedDeposit;
        //   assert(u128.gt(amount, u128.Zero), '`attachedDeposit` must be > 0.');
        //   const donor = Context.sender;
        // let escrow_contract_name = env::current_account_id(); // https://docs.near.org/develop/contracts/environment/
        //   const prepaidGas = Context.prepaidGas;
        //   const gasAlreadyBurned = Context.usedGas;
        //   const gasToBeBurnedDuringTransferFromEscrow = XCC_GAS;
        //   const remainingGas = prepaidGas - gasAlreadyBurned - gasToBeBurnedDuringTransferFromEscrow;
        //   logging.log(
        //     `prepaidGas=${prepaidGas}, gasAlreadyBurned=${gasAlreadyBurned}, gasToBeBurnedDuringTransferFromEscrow=${gasToBeBurnedDuringTransferFromEscrow}, remainingGas=${remainingGas}`,
        //   );
        //   _transfer_from_escrow(recipient, amount) // Immediately pass it along.
        //     .then(escrowContractName)
        //     .function_call<DRAE>('transfer_from_escrow_callback_after_donating', { donor, recipient, amount, escrowContractName }, u128.Zero, remainingGas);
    }

    pub fn delete_all_matches_associated_with_recipient(&mut self, recipient: AccountId) -> String {
        // TODO assert_self();
        match self.recipients.get(&recipient) {
            Some(matchers_for_this_recipient) => {
                // let temp = JsonContainer {
                //     matcher_account_map: matchers_for_this_recipient,
                // };
                let json_value = matchers_for_this_recipient
                    .iter()
                    .map(|(k, v)| (k, v.to_string()))
                    .collect::<serde_json::Value>(); //serde_json::to_string(&temp).unwrap();
                let existing_commitments_from_matchers =
                    serde_json::to_string(&json_value).unwrap();
                self.recipients.remove(&recipient);
                format!(
                    "Recipient '{}' had these matchers, which are now deleted: {}",
                    recipient, existing_commitments_from_matchers
                )
            }
            None => {
                format!(
                    "Recipient '{}' did not have any matchers to delete.",
                    recipient
                )
            }
        }
    }
}
