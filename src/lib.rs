// Using https://github.com/near-examples/docs-examples/blob/4fda29c8cdabd9aba90787c553413db7725d88bd/donation-rs/contract/src/lib.rs as a basis

use helpers::generic::{did_promise_succeed, hash_account_id, near_string_to_yocto};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap};
use near_sdk::{
    env, log, near_bindgen, AccountId, Balance, BorshStorageKey, CryptoHash, Gas,
    PanicOnDefault, Promise,
};
use std::cmp;
use std::collections::HashMap;
use witgen::witgen;
use near_units::{near};

mod helpers;
use crate::generic::yocto_to_near_string;
pub use crate::helpers::generic;

#[witgen]
type Amount = Balance;
type MatcherAccountId = AccountId;
type MatcherAmountMap = UnorderedMap<MatcherAccountId, Amount>; // https://doc.rust-lang.org/reference/items/type-aliases.html
type InMemoryMatcherAmountMap = HashMap<MatcherAccountId, Amount>;
type RecipientAccountId = AccountId;
type MatcherAmountPerRecipient = LookupMap<RecipientAccountId, MatcherAmountMap>;

pub const STORAGE_COST: Amount = 1_000_000_000_000_000_000_000; // ONEDAY: Write this in a more human-readable way, and document how this value was decided.
pub const GAS_FOR_ACCOUNT_CALLBACK: Gas = Gas(20_000_000_000_000); // gas for cross-contract calls, ~5 Tgas (teragas = 1e12) per "hop"

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

// ONEDAY: Review each part of this repo to ensure that it can scale to large amounts of data.

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

    #[private]
    fn create_new_matcher_amount_map(recipient: &AccountId) -> MatcherAmountMap {
        MatcherAmountMap::new(StorageKey::RecipientsInner {
            hash: hash_account_id(&recipient.to_string()),
        })
    }

    #[private]
    fn get_expected_matchers_for_this_recipient(&self, recipient: &AccountId) -> MatcherAmountMap {
        let msg = format!("Could not find any matchers for recipient `{}`", &recipient);
        self.recipients.get(recipient).expect(&msg)
    }

    #[private]
    fn get_expected_commitment(
        &self,
        recipient: &RecipientAccountId,
        matchers_for_this_recipient: &MatcherAmountMap,
        matcher: &AccountId,
    ) -> Amount {
        let existing_commitment = matchers_for_this_recipient.get(matcher).unwrap_or_else(|| panic!("{} does not currently have any funds committed to {}.",
                matcher, recipient));
        near_sdk::log!(
            "existing_commitment = {}",
            near::to_human(existing_commitment)
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
         format!(
            "These matchers are committed to match donations to {} up to a maximum of the following amounts:\n{}",
            recipient,
            matchers_log.join("\n")
            )
    }

    pub fn transfer_from_escrow(&self, destination_account: &AccountId, amount: Amount) -> Promise {
        // ONEDAY: Consider subtracting storage cost like https://github.com/near-examples/docs-examples/blob/4fda29c8cdabd9aba90787c553413db7725d88bd/donation-rs/contract/src/lib.rs#L51
        log!(
            "transfer_from_escrow destination_account: {}, amount: {}",
            destination_account,
            yocto_to_near_string(amount)
        );
        Promise::new(destination_account.clone()).transfer(amount) // https://www.near-sdk.io/cross-contract/callbacks#calculator-example uses .clone()
    }

    /**
     * Gets called via `rescind_matching_funds` and `send_matching_donation`.
     */
    #[private]
    fn set_matcher_amount(
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
        // ONEDAY assert_self(); assert_single_promise_success();
        let mut matchers_for_this_recipient =
            self.get_expected_matchers_for_this_recipient(recipient);
        if amount > 0 {
            let _existing_commitment =
                self.get_expected_commitment(recipient, &matchers_for_this_recipient, matcher); // ONEDAY Assert that there is a matcher?
            matchers_for_this_recipient.insert(matcher, &amount);
        } else {
            self.recipients.remove(matcher);
        }

        matchers_for_this_recipient
    }

    #[private] // Public - but only callable by env::current_account_id()
    pub fn on_rescind_matching_funds(
        &mut self,
        recipient: &AccountId,
        matcher: AccountId,
        original_amount: Amount,
    ) {
        if !did_promise_succeed() {
            // If transfer failed, change the state back to what it was:
            self.set_matcher_amount(recipient, &matcher, original_amount);
        }
    }

    /// requested_withdrawal_amount is in NEAR (commas, underscores, and 'Ⓝ' are acceptable and will be ignored)
    pub fn rescind_matching_funds(
        &mut self,
        recipient: &AccountId,
        requested_withdrawal_amount: generic::FormattedNearString,
    ) -> String {
        let matcher = env::signer_account_id();
        let matchers_for_this_recipient = self.get_expected_matchers_for_this_recipient(recipient);
        let amount_already_committed =
            self.get_expected_commitment(recipient, &matchers_for_this_recipient, &matcher);
        let requested_withdrawal_amount_yocto: Amount = near_string_to_yocto(requested_withdrawal_amount);
        let result;
        let mut amount_to_decrease = requested_withdrawal_amount_yocto;
        let mut new_amount = 0;
        if requested_withdrawal_amount_yocto > amount_already_committed {
            amount_to_decrease = amount_already_committed;
            result =
                format!(
                "{} is about to rescind {} and then will not be matching donations to {} anymore",
                &matcher, yocto_to_near_string(amount_to_decrease), recipient
            );
        } else {
            new_amount = amount_already_committed - amount_to_decrease;
            result = format!(
                "{} is about to rescind {} and then will only be committed to match donations to {} up to a maximum of {}.",
                 &matcher, 
                 yocto_to_near_string(amount_to_decrease), 
                 recipient, 
                 yocto_to_near_string(new_amount)
            );
        }
        self.set_matcher_amount(recipient, &matcher, new_amount);
        self.transfer_from_escrow(&matcher, amount_to_decrease) // Funds go from escrow back to the matcher.
            .then(
                Self::ext(env::current_account_id()) // escrow contract name
                    .with_static_gas(GAS_FOR_ACCOUNT_CALLBACK)
                    .on_rescind_matching_funds(recipient, matcher, amount_already_committed),
            );
        result
    }

    // Only gets called internally by send_matching_donations.
    #[private]
    fn record_matching_donations_as_sent(        &mut self,        donation_amount: &Amount ,        recipient: &AccountId,            ) ->(Amount,InMemoryMatcherAmountMap){
        let mut sum_of_donations_to_send = donation_amount.clone();
        // TODO optimistically change state, then do the actual transfer, then in the callback undo the state change if the transfer failed
        let mut matchers_for_this_recipient: MatcherAmountMap =
            self.get_expected_matchers_for_this_recipient(&recipient);
            let mut original_commitments=InMemoryMatcherAmountMap::new();
        let matchers = matchers_for_this_recipient.keys_as_vector();        
        for matcher in matchers.iter() {
            let existing_commitment =
            self.get_expected_commitment(recipient, &matchers_for_this_recipient, &matcher);
            let matched_amount: u128 = cmp::min(donation_amount.clone(), existing_commitment);
            let remaining_commitment: u128 = existing_commitment - matched_amount;
            near_sdk::log!(
                "{} will send a matching donation of {} to {}. Remaining commitment: {}.",
    &           &matcher,
                yocto_to_near_string(matched_amount),
                &recipient,
                yocto_to_near_string(remaining_commitment)
            );
            matchers_for_this_recipient.insert(&matcher, &matched_amount);
            original_commitments.insert(matcher, existing_commitment);
            sum_of_donations_to_send+=matched_amount;
        }
        
        (sum_of_donations_to_send,original_commitments)
    }

    #[private] // Public - but only callable by env::current_account_id()
    pub fn on_donate(        &mut self,        donation_amount: &Amount,         original_commitments: &InMemoryMatcherAmountMap,    ) {
        if !did_promise_succeed() {
            // If transfer failed, change the state back to what it was:
            // TODO (and send funds back to donor?)
            // TODO Do it for every matcher of this recipient
            
        }
    }

    #[payable] // Public - People can attach money
    pub fn donate(&mut self, recipient: AccountId) {
        let donation_amount: Amount = env::attached_deposit();
        assert!(donation_amount > 0, "Attaching some yoctoNEAR is required.");
        let prepaid_gas = env::prepaid_gas();
        let gas_already_burned = env::used_gas();
        let gas_to_be_burned_during_transfer_from_escrow = GAS_FOR_ACCOUNT_CALLBACK;
        let remaining_gas =
            prepaid_gas - gas_already_burned - gas_to_be_burned_during_transfer_from_escrow;
        near_sdk::log!(
            "prepaid_gas={:?}, gas_already_burned={:?}, gas_to_be_burned_during_transfer_from_escrow={:?}, remaining_gas={:?}",
            prepaid_gas,
            gas_already_burned,
            gas_to_be_burned_during_transfer_from_escrow,   // TODO Why is Prettier not working?
            remaining_gas
          );
        let (sum_of_donations_to_send, original_commitments)= self.record_matching_donations_as_sent(&donation_amount,&recipient);
        self.transfer_from_escrow(&recipient, sum_of_donations_to_send) // The donor attached a deposit which this contract owns at this point. Immediately pass it along to the intended recipient.
        .then(
            Self::ext(env::current_account_id()) // escrow contract name
        .with_static_gas(GAS_FOR_ACCOUNT_CALLBACK)
                .on_donate(&donation_amount, &original_commitments));
    }

    #[private] // Public - but only callable by env::current_account_id()
    pub fn delete_all_matches_associated_with_recipient(&mut self, recipient: AccountId) {
        // Since self.recipients is a LookupMap (not iterable), there is no clear() function available for instantly deleting all keys.
        // ONEDAY assert_self();
            let mut matchers_for_this_recipient: MatcherAmountMap =
            self.get_expected_matchers_for_this_recipient(&recipient);
        let matchers = matchers_for_this_recipient.keys_as_vector();
        let mut to_remove = Vec::new();
        for matcher in matchers.iter() {
            to_remove.push(matcher); 
        }
        for key in to_remove.iter(){// https://stackoverflow.com/a/45724774/470749
            matchers_for_this_recipient.remove(key); // If not for this loop, the contract state would be messed up, and we would later get "The collection is an inconsistent state" errors.
            near_sdk::log!("Removed {} from {}", &key, &recipient);
        }
        self.recipients.remove(&recipient); // See comment above about why removing each inner map is also necessary.
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};

    use crate::generic::{yocto_to_near_string};

    use super::*;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_yocto_to_near() {
        // let mut context = get_context(accounts(2));
        // testing_env!(context.build());
        
        assert_eq!(yocto_to_near_string(3_193_264_587_249_763_651_824_729), "3.1932 Ⓝ"); // https://docs.rs/near-helper/latest/near_helper/fn.yoctonear_to_near.html
        assert_eq!(yocto_to_near_string(21_409_258_000_000_000_000_000), "0.0214 Ⓝ"); // https://docs.rs/near-helper/latest/near_helper/fn.yoctonear_to_near.html
        assert_eq!(yocto_to_near_string(10_000_000_000_000_000_000_000), "0.01 Ⓝ");
        assert_eq!(yocto_to_near_string(700_000_000_000_000_000_000), "0.0007 Ⓝ");
    }

    #[test]
    fn test_near_string_to_yocto() {
        assert_eq!(near_string_to_yocto("3.997 Ⓝ".to_string()), 3_997_000_000_000_000_000_000_000);
        assert_eq!(near_string_to_yocto("0.018 Ⓝ".to_string()), 18_000_000_000_000_000_000_000);
    }
}