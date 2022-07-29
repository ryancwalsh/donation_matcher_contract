// https://www.near-sdk.io/testing/unit-tests

#[cfg(all(test, not(target_arch = "wasm32")))]
mod lib_tests {
    use crate::generic::near_string_to_yocto;
    use crate::{Amount, Contract};

    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    fn set_context(account_index: usize, is_view: bool, deposit: Amount) {
        let context = VMContextBuilder::new()
            .signer_account_id(accounts(account_index))
            .is_view(is_view)
            .attached_deposit(deposit)
            .build();
        // let context_builder = VMContextBuilder::new()
        //     .signer_account_id(accounts(account_index))
        //     .is_view(is_view);
        // match deposit_near {
        //     Some(near_str) => {
        //         let deposit_yocto = near_string_to_yocto(near_str.to_string());
        //         context_builder.attached_deposit(deposit_yocto);
        //     }
        //     None => {
        //         // do nothing
        //     }
        // }
        // let context = context_builder.build();
        testing_env!(context);
    }

    #[test]
    fn test_offer_matching_funds_and_get_commitments() {
        let mut contract = Contract::new();
        set_context(1, false, near_string_to_yocto("0.3".to_string()));
        let recipient = accounts(0);
        let _matcher1_offer_result = contract.offer_matching_funds(&recipient);
        set_context(2, false, near_string_to_yocto("0.1".to_string()));
        let _matcher2_offer_result = contract.offer_matching_funds(&recipient);
        // TODO: Assert that this (escrow) contract now contains the correct amount of funds. Assert that the matchers' account balances have decreased appropriately.
        let result = contract.get_commitments(&recipient);
        assert_eq!(result, "These matchers are committed to match donations to alice up to a maximum of the following amounts:\nbob: 0.3 Ⓝ,\ncharlie: 0.1 Ⓝ,".to_string());
        set_context(1, false, 0);
        let _matcher1_rescind_result =
            contract.rescind_matching_funds(&recipient, "0.02 Ⓝ".to_string());
        // TODO: Assert funds received via transfer. Check state.
        let result_after_rescind = contract.get_commitments(&recipient);
        assert_eq!(result_after_rescind, "These matchers are committed to match donations to alice up to a maximum of the following amounts:\nbob: 0.28 Ⓝ,\ncharlie: 0.1 Ⓝ,".to_string());
    }
}
