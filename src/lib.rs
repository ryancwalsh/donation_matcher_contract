// Using https://github.com/near-examples/docs-examples/blob/4fda29c8cdabd9aba90787c553413db7725d88bd/donation-rs/contract/src/lib.rs as a basis

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, log, near_bindgen, AccountId, Balance, Promise};

pub const STORAGE_COST: u128 = 1_000_000_000_000_000_000_000; // ONEDAY: Write this in a more human-readable way, and document how this value was decided.

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
    pub matcher_account_id_commitment_amount:
        UnorderedMap<AccountId, UnorderedMap<AccountId, u128>>, // https://docs.near.org/concepts/storage/data-storage#unorderedmap The outer key-value pair is the "recipient: matcher-amount-map". The inner map (matcher amount) has a key-value pair of "matcher: amount".
}

#[near_bindgen]
impl Contract {
    #[init]
    #[private] // Public - but only callable by env::current_account_id()
    pub fn new() -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            matcher_account_id_commitment_amount: UnorderedMap::new(b"d"),
        }
    }

    #[payable] // Public - People can attach money
    pub fn offer_matching_funds(&mut self, recipient: AccountId) -> String {
        let donation_amount: Balance = env::attached_deposit();
        assert!(
            donation_amount > STORAGE_COST,
            "Attach at least {} yoctoNEAR",
            STORAGE_COST
        );
        let matcher = env::signer_account_id(); // https://docs.near.org/develop/contracts/environment/
        let mut matchers_for_this_recipient =
            match self.matcher_account_id_commitment_amount.get(&recipient) {
                Some(matcher_commitment_map) => matcher_commitment_map,
                None => {
                    let prefix = b"m"; // TODO: How to decide this prefix?
                    UnorderedMap::new(prefix)
                }
            };
        let mut total = donation_amount;
        match matchers_for_this_recipient.get(&matcher) {
            Some(existing_commitment) => {
                total += existing_commitment;
                matchers_for_this_recipient.remove(&matcher);
            }
            None => {}
        }
        matchers_for_this_recipient.insert(&matcher, &total);
        let result = format!(
            "{} is now committed to match donations to {} up to a maximum of {}.",
            matcher, recipient, total
        );
        log!(result);
        result
    }

    /*pub fn get_commitments(&mut self, account_id: AccountId) -> String {
        //       const matchersLog: string[] = [];
        // const matchersForThisRecipient = _getMatcherCommitmentsToRecipient(recipient);
        // const matchers = matchersForThisRecipient.keys();
        // for (let i = 0; i < matchers.length; i += 1) {
        //   const matcher = matchers[i];
        // &  const existingCommitment: u128 = matchersForThisRecipient.getSome(matcher);
        //   const msg = `${matcher} is committed to match donations to ${recipient} up to a maximum of ${existingCommitment.toString()}.`;
        //   logging.log(msg);
        //   matchersLog.push(msg);
        // }
        // return matchersLog.join(' ');
    }*/

    pub fn transfer_from_escrow(&self, destination_account: AccountId, amount: u128) -> Promise {
        // TODO: Consider subtracting storage cost like https://github.com/near-examples/docs-examples/blob/4fda29c8cdabd9aba90787c553413db7725d88bd/donation-rs/contract/src/lib.rs#L51
        log!(
            "transfer_from_escrow destination_account: {}, amount: {}",
            destination_account,
            amount
        );
        Promise::new(destination_account.clone()).transfer(amount)
    }
}
