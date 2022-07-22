// Using https://github.com/near-examples/docs-examples/blob/4fda29c8cdabd9aba90787c553413db7725d88bd/donation-rs/contract/src/lib.rs as a basis

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, log, near_bindgen, serde_json, AccountId, Balance, PanicOnDefault, Promise};

type Amount = Balance;
type MatcherAccountId = AccountId;
type MatcherAmountMap = UnorderedMap<MatcherAccountId, Amount>; // https://doc.rust-lang.org/reference/items/type-aliases.html
type RecipientAccountId = AccountId;
type MatcherAmountPerRecipient = UnorderedMap<RecipientAccountId, MatcherAmountMap>;

pub const STORAGE_COST: Amount = 1_000_000_000_000_000_000_000; // ONEDAY: Write this in a more human-readable way, and document how this value was decided.

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
            recipients: MatcherAmountPerRecipient::new(b"d"),
        }
    }

    fn create_new_matcher_amount_map(recipient: &AccountId) -> MatcherAmountMap {
        let prefix_string = "r".to_string() + &recipient.to_string();
        let prefix: &[u8] = prefix_string.as_bytes();
        MatcherAmountMap::new(prefix)
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
        let mut matchers_for_this_recipient = match self.recipients.get(&recipient) {
            Some(matcher_commitment_map) => matcher_commitment_map,
            None => Self::create_new_matcher_amount_map(&recipient),
        };
        let mut total = donation_amount;
        match matchers_for_this_recipient.get(&matcher) {
            Some(existing_commitment) => {
                total = total + existing_commitment;
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

    pub fn get_commitments(&mut self, recipient: AccountId) -> String {
        let mut matchers_log: Vec<String> = Vec::new();
        let matchers_for_this_recipient = match self.recipients.get(&recipient) {
            Some(matcher_commitment_map) => matcher_commitment_map,
            None => Self::create_new_matcher_amount_map(&recipient),
        };
        let matchers = matchers_for_this_recipient.keys_as_vector();
        let mut index = 0;
        while index < matchers.len() {
            let matcher = matchers.get(index).unwrap();
            let existing_commitment = matchers_for_this_recipient.get(&matcher).unwrap();
            let msg = format!(
                "{} is committed to match donations to {} up to a maximum of {}.",
                matcher, recipient, existing_commitment
            );
            log!(msg);
            matchers_log.push(msg);
            index += 1;
        }
        matchers_log.join(" ")
    }

    pub fn transfer_from_escrow(&self, destination_account: AccountId, amount: Amount) -> Promise {
        // TODO: Consider subtracting storage cost like https://github.com/near-examples/docs-examples/blob/4fda29c8cdabd9aba90787c553413db7725d88bd/donation-rs/contract/src/lib.rs#L51
        log!(
            "transfer_from_escrow destination_account: {}, amount: {}",
            destination_account,
            amount
        );
        Promise::new(destination_account.clone()).transfer(amount)
    }

    /**
     * Gets called via `rescind_matching_funds` and `send_matching_donation`.
     */
    fn set_matcher_amount(
        &mut self,
        recipient: AccountId,
        matcher: AccountId,
        amount: Amount,
    ) -> MatcherAmountMap {
        //logging.log(`setMatcherAmount(recipient: ${recipient}, matcher: ${matcher}, amount: ${amount})`);
        // TODO assert_self();
        // TODO assert_single_promise_success();
        let mut matchers_for_this_recipient = match self.recipients.get(&recipient) {
            Some(matcher_commitment_map) => matcher_commitment_map,
            None => Self::create_new_matcher_amount_map(&recipient), // How would this line ever be reached?
        };
        if amount > 0 {
            match matchers_for_this_recipient.get(&matcher) {
                Some(_) => {
                    matchers_for_this_recipient.remove(&matcher);
                    matchers_for_this_recipient.insert(&matcher, &amount);
                }
                None => {} // How would this line ever be reached?
            }
        } else {
            self.recipients.remove(&matcher);
        }

        matchers_for_this_recipient
    }

    pub fn rescind_matching_funds(
        &mut self,
        recipient: AccountId,
        requested_withdrawal_amount: Amount,
    ) -> String {
        let escrow_contract_name = env::current_account_id(); // https://docs.near.org/develop/contracts/environment/
        let matcher = env::signer_account_id();
        let matchers_for_this_recipient = match self.recipients.get(&recipient) {
            Some(matcher_commitment_map) => matcher_commitment_map,
            None => Self::create_new_matcher_amount_map(&recipient), // How would this line ever be reached?
        };
        let result;
        match matchers_for_this_recipient.get(&matcher) {
            Some(amount_already_committed) => {
                let mut amount_to_decrease = requested_withdrawal_amount;
                let mut new_amount = 0;
                if requested_withdrawal_amount > amount_already_committed {
                    amount_to_decrease = amount_already_committed;
                    result = format!("{} is about to rescind {} and then will not be matching donations to {} anymore", matcher, amount_to_decrease, recipient);
                } else {
                    new_amount = amount_already_committed - amount_to_decrease;
                    result = format!("{} is about to rescind {} and then will only be committed to match donations to {} up to a maximum of {}.", matcher, amount_to_decrease, recipient, new_amount);
                }
                // TODO transfer_from_escrow(matcher, amount_to_decrease) // Funds go from escrow back to the matcher.
                //       .then(escrow_contract_name)
                //       .function_call<RecipientMatcherAmount>('setMatcherAmount', { recipient, matcher, amount: new_amount }, u128.Zero, XCC_GAS);
                // }
            }
            None => {
                result = format!("{} does not currently have any funds committed to {}, so funds cannot be rescinded.", matcher, recipient);
            }
        }

        result
    }

    fn send_matching_donation(
        matcher: AccountId,
        recipient: AccountId,
        amount: Amount,
        matchers_for_this_recipient: MatcherAmountMap,
    ) {
        //   const currentCommitment: u128 = matchersForThisRecipient.getSome(matcher);
        //   const matchedAmount: u128 = min(amount, currentCommitment);
        //   const remainingCommitment: u128 = u128.sub(currentCommitment, matchedAmount);
        //   logging.log(`${matcher} will send a matching donation of ${matchedAmount} to ${recipient}. Remaining commitment: ${remainingCommitment}.`);
        //   _transferFromEscrow(recipient, matchedAmount)
        //     .then(escrowContractName)
        //     .function_call<RecipientMatcherAmount>('setMatcherAmount', { recipient, matcher, amount: remainingCommitment }, u128.Zero, XCC_GAS);
    }

    fn send_matching_donations(recipient: AccountId, amount: Amount) {
        //   const matchersForThisRecipient = _getMatcherCommitmentsToRecipient(recipient);
        //   const matcherKeysForThisRecipient = matchersForThisRecipient.keys();
        //   for (let i = 0; i < matcherKeysForThisRecipient.length; i += 1) {
        //     const matcher = matcherKeysForThisRecipient[i];
        //     _sendMatchingDonation(matcher, recipient, amount, matchersForThisRecipient, escrowContractName);
        //   }
    }

    pub fn transfer_from_escrow_callback_after_donating(
        donor: AccountId,
        recipient: AccountId,
        amount: Amount,
    ) {
        //   assert_self();
        //   assert_single_promise_success();

        //   logging.log(`transferFromEscrowCallbackAfterDonating. ${donor} donated ${amount} to ${recipient}.`);
        //   _sendMatchingDonations(recipient, amount, escrowContractName);
    }

    pub fn donate(recipient: AccountId) {
        //   const amount = Context.attachedDeposit;
        //   assert(u128.gt(amount, u128.Zero), '`attachedDeposit` must be > 0.');
        //   const donor = Context.sender;
        //   const escrowContractName = Context.contractName;
        //   const prepaidGas = Context.prepaidGas;
        //   const gasAlreadyBurned = Context.usedGas;
        //   const gasToBeBurnedDuringTransferFromEscrow = XCC_GAS;
        //   const remainingGas = prepaidGas - gasAlreadyBurned - gasToBeBurnedDuringTransferFromEscrow;
        //   logging.log(
        //     `prepaidGas=${prepaidGas}, gasAlreadyBurned=${gasAlreadyBurned}, gasToBeBurnedDuringTransferFromEscrow=${gasToBeBurnedDuringTransferFromEscrow}, remainingGas=${remainingGas}`,
        //   );
        //   _transferFromEscrow(recipient, amount) // Immediately pass it along.
        //     .then(escrowContractName)
        //     .function_call<DRAE>('transferFromEscrowCallbackAfterDonating', { donor, recipient, amount, escrowContractName }, u128.Zero, remainingGas);
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
