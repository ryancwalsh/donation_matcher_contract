// https://www.near-sdk.io/testing/unit-tests

#[cfg(all(test, not(target_arch = "wasm32")))]
mod lib_tests {
    use crate::generic::{near_string_to_yocto, yocto_to_near_string};
    use crate::{Amount, Contract};

    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{env, log, testing_env, Balance};

    fn set_context(
        account_index: usize,
        is_view: bool,
        starting_balance: Balance,
        deposit: Amount,
    ) {
        let context = VMContextBuilder::new()
            .signer_account_id(accounts(account_index))
            .is_view(is_view)
            .account_balance(starting_balance) // TODO: Change this such that set_context isn't always resetting the balance.
            .attached_deposit(deposit)
            .build();
        testing_env!(context);
    }

    fn log_balance() {
        log!(
            "⬜⬜⬜⬜ account_balance: {:?}: {:?}",
            env::current_account_id(),
            yocto_to_near_string(&env::account_balance())
        );
    }

    #[test]
    fn test_offer_matching_funds_and_get_commitments_and_rescind_matching_funds_and_donate() {
        let mut contract = Contract::new();
        let recipient = accounts(0); // 0 = Alice
        set_context(
            0, // 0 = Alice
            false,
            near_string_to_yocto(&"1".to_string()),
            0,
        );
        //log_balance();
        set_context(
            1, // Bob = 1
            false,
            near_string_to_yocto(&"1".to_string()),
            near_string_to_yocto(&"0.3".to_string()),
        );
        log_balance();
        let _matcher1_offer_result = contract.offer_matching_funds(&recipient);
        log_balance();
        set_context(
            2, // Charlie = 2
            false,
            near_string_to_yocto(&"1".to_string()),
            near_string_to_yocto(&"0.1".to_string()),
        );
        let _matcher2_offer_result = contract.offer_matching_funds(&recipient);
        // TODO: Assert that this (escrow) contract now contains the correct amount of funds. Assert that the matchers' account balances have decreased appropriately.
        log_balance();
        let result = contract.get_commitments(&recipient);
        assert_eq!(
            result,
            "{\"bob\":\"0.3 Ⓝ\",\"charlie\":\"0.1 Ⓝ\"}".to_string()
        );
        set_context(1, false, near_string_to_yocto(&"1".to_string()), 0);
        let _matcher1_rescind_result =
            contract.rescind_matching_funds(&recipient, "0.02 Ⓝ".to_string());
        // TODO: Assert funds received via transfer. Check state.
        let result_after_rescind = contract.get_commitments(&recipient);
        assert_eq!(
            result_after_rescind,
            "{\"bob\":\"0.28 Ⓝ\",\"charlie\":\"0.1 Ⓝ\"}".to_string()
        );
        set_context(
            3, // Danny = 3
            false,
            near_string_to_yocto(&"1".to_string()),
            near_string_to_yocto(&"0.1".to_string()),
        );
        contract.donate(&recipient);
        // let result_after_donation = contract.get_commitments(&recipient);
        // assert_eq!(result_after_donation, "{bob: 0.18 Ⓝ,".to_string());
    }
}
