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
            "account_balance: {:?}: {:?}",
            env::current_account_id(),
            yocto_to_near_string(&env::account_balance())
        );
    }

    #[test]
    fn test_offer_matching_funds_and_get_commitments_and_rescind_matching_funds() {
        let mut contract = Contract::new();
        let starting_balance = near_string_to_yocto(&"1".to_string());
        let recipient = accounts(0); // 0 = Alice
        set_context(
            0, // 0 = Alice
            false,
            starting_balance,
            0,
        );
        //log_balance();
        set_context(
            1, // 1 = Bob
            false,
            starting_balance,
            near_string_to_yocto(&"0.3".to_string()),
        );
        log_balance();
        let _matcher1_offer_result = contract.offer_matching_funds(&recipient);
        log_balance();
        set_context(
            2, // 2 = Charlie
            false,
            starting_balance,
            near_string_to_yocto(&"0.1".to_string()),
        );
        let _matcher2_offer_result = contract.offer_matching_funds(&recipient);
        // Unit tests cannot assert that this (escrow) contract now contains the correct amount of funds. The integration tests should do that and also assert that the matchers' account balances have decreased appropriately.
        log_balance();
        let result = contract.get_commitments(&recipient);
        assert_eq!(
            result,
            "{\"bob\":\"0.3 Ⓝ\",\"charlie\":\"0.1 Ⓝ\"}".to_string()
        );
        set_context(1, false, starting_balance, 0);
        let _matcher1_rescind_result =
            contract.rescind_matching_funds(&recipient, "0.02 Ⓝ".to_string());
        // Unit tests cannot assert funds received via transfer (check state). The integration tests should.
        let result_after_rescind = contract.get_commitments(&recipient);
        assert_eq!(
            result_after_rescind,
            "{\"bob\":\"0.28 Ⓝ\",\"charlie\":\"0.1 Ⓝ\"}".to_string()
        );
        set_context(
            3, // 3 = Danny
            false,
            starting_balance,
            near_string_to_yocto(&"0.1".to_string()),
        );
    }

    #[test]
    fn test_repeated_offers_and_rescinds() {
        let mut contract = Contract::new();
        let starting_balance = near_string_to_yocto(&"1".to_string());
        let recipient = accounts(0); // 0 = Alice
        set_context(
            0, // 0 = Alice
            false,
            starting_balance,
            0,
        );
        //log_balance();
        set_context(
            1, // 1 = Bob
            false,
            starting_balance,
            near_string_to_yocto(&"0.1".to_string()),
        );
        log_balance();
        let _matcher1_offer_result = contract.offer_matching_funds(&recipient);
        log_balance();
        set_context(
            1, // 1 = Bob
            false,
            starting_balance,
            near_string_to_yocto(&"0.1".to_string()),
        );
        let _matcher2_offer_result = contract.offer_matching_funds(&recipient);
        // Unit tests cannot assert that this (escrow) contract now contains the correct amount of funds. The integration tests should do that and also assert that the matchers' account balances have decreased appropriately.
        log_balance();
        let result = contract.get_commitments(&recipient);
        assert_eq!(result, "{\"bob\":\"0.2 Ⓝ\"}".to_string());
        set_context(1, false, starting_balance, 0);
        let _matcher1_rescind_result1 =
            contract.rescind_matching_funds(&recipient, "0.02 Ⓝ".to_string());
        // Unit tests cannot assert funds received via transfer (check state). The integration tests should.
        let result_after_rescind1 = contract.get_commitments(&recipient);
        assert_eq!(result_after_rescind1, "{\"bob\":\"0.18 Ⓝ\"}".to_string());
        let _matcher1_rescind_result2 =
            contract.rescind_matching_funds(&recipient, "99 Ⓝ".to_string());
        // Unit tests cannot assert funds received via transfer (check state). The integration tests should.
        let result_after_rescind2 = contract.get_commitments(&recipient);
        assert_eq!(result_after_rescind2, "{}".to_string());
    }

    #[test]
    fn test_offer_matching_funds_and_donate_and_get_commitments() {
        let mut contract = Contract::new();
        let starting_balance = near_string_to_yocto(&"1".to_string());
        let offer = near_string_to_yocto(&"0.3".to_string());
        let donation = near_string_to_yocto(&"0.2".to_string());
        let recipient = accounts(0); // 0 = Alice
        set_context(
            0, // 0 = Alice
            false,
            starting_balance,
            0,
        );
        set_context(
            1, // 1 = Bob
            false,
            starting_balance,
            offer,
        );
        log_balance();
        let _matcher1_offer_result = contract.offer_matching_funds(&recipient);

        log_balance();

        set_context(2, false, starting_balance, donation);
        let _donate_result = contract.donate(&recipient);
        // Unit tests cannot assert funds received via transfer (check state). The integration tests should.
        let commitments_after_donate = contract.get_commitments(&recipient);
        assert_eq!(
            commitments_after_donate,
            format!(
                "{{\"bob\":\"{}\"}}",
                yocto_to_near_string(&(offer - donation))
            )
        );
    }
}
