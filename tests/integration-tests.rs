// https://github.com/near/workspaces-rs/blob/8f12f3dc3b0251ac3f44ddf6ab6fc63003579139/workspaces/tests/create_account.rs

#![recursion_limit = "256"]

use anyhow::Error;
use donation_matcher_contract::{
    generic::{near_string_to_yocto, yocto_to_near_string},
    GAS_FOR_ACCOUNT_CALLBACK,
};
use near_sdk::{serde_json::json, Balance};
use test_log::test;
use workspaces::{network::Sandbox, prelude::*, Account, Worker};

async fn create_subaccount(
    worker: &Worker<Sandbox>,
    parent_account: &Account,
    name: &str,
    initial_balance: &str,
) -> Result<Account, Error> {
    let subaccount = parent_account
        .create_subaccount(&worker, name)
        .initial_balance(near_string_to_yocto(&initial_balance.to_string()))
        .transact()
        .await?
        .into_result()?;

    Ok(subaccount)
}

fn assert_approx_considering_gas(amount1: &Balance, amount2: &Balance) {
    const TOLERANCE: &str = &"0.0061 Ⓝ";
    let tolerance: Balance = near_string_to_yocto(&TOLERANCE.to_string());
    assert!(amount1 <= amount2);
    near_sdk::log!("amount1 {} <= amount2 {}", amount1, amount2);
    near_sdk::log!("tolerance = {}", tolerance);
    assert!(
        amount1 >= &(*amount2 - &tolerance),
        "Check whether gas used <= the tolerance specified in this assertion. {}. Formatted: {}",
        amount2 - amount1,
        yocto_to_near_string(&(amount2 - amount1))
    );
}

#[test(tokio::test)]
async fn test_offer_matching_funds_and_get_commitments_and_rescind_matching_funds_and_donate(
) -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let contract = worker
        .dev_deploy(&include_bytes!("../target/res/donation_matcher_contract.wasm").to_vec())
        .await?;
    let parent_account = worker.dev_create_account().await?;
    contract
        .call(&worker, "new")
        //.args_json(json!({"recipient": &recipient.id()}))?
        .gas(GAS_FOR_ACCOUNT_CALLBACK.0)
        .transact()
        .await?;

    let starting_balance_for_each_acct = "1 Ⓝ".to_string();
    let matcher1_offer = "0.3 Ⓝ".to_string();
    let matcher2_offer = "0.1 Ⓝ".to_string();
    let matcher1_rescind = "0.02 Ⓝ".to_string();

    let recipient = create_subaccount(
        &worker,
        &parent_account,
        "recipient",
        starting_balance_for_each_acct.as_str(),
    )
    .await?;

    assert_eq!(
        yocto_to_near_string(&recipient.view_account(&worker).await?.balance),
        starting_balance_for_each_acct
    );

    let matcher1 = create_subaccount(
        &worker,
        &parent_account,
        "matcher1",
        starting_balance_for_each_acct.as_str(),
    )
    .await?;

    let _matcher1_offer_result = matcher1
        .call(&worker, contract.id(), "offer_matching_funds")
        .args_json(json!({"recipient": &recipient.id()}))?
        .gas(GAS_FOR_ACCOUNT_CALLBACK.0)
        .deposit(near_string_to_yocto(&matcher1_offer.to_string()))
        .transact()
        .await?;

    assert_eq!(
        yocto_to_near_string(&recipient.view_account(&worker).await?.balance),
        starting_balance_for_each_acct
    ); // The recipient hasn't received any donation yet.
    let matcher1_bal_after_offer = &near_string_to_yocto(&starting_balance_for_each_acct)
        - &near_string_to_yocto(&matcher1_offer.to_string());
    assert_approx_considering_gas(
        &matcher1.view_account(&worker).await?.balance,
        &matcher1_bal_after_offer,
    );

    let matcher2 = create_subaccount(
        &worker,
        &parent_account,
        "matcher2",
        starting_balance_for_each_acct.as_str(),
    )
    .await?;

    let _matcher2_offer_result = matcher2
        .call(&worker, contract.id(), "offer_matching_funds")
        .args_json(json!({"recipient": &recipient.id()}))?
        .gas(GAS_FOR_ACCOUNT_CALLBACK.0)
        .deposit(near_string_to_yocto(&matcher2_offer.to_string()))
        .transact()
        .await?;

    assert_eq!(
        yocto_to_near_string(&recipient.view_account(&worker).await?.balance),
        starting_balance_for_each_acct
    ); // The recipient hasn't received any donation yet.
    let matcher2_bal_after_offer = &near_string_to_yocto(&starting_balance_for_each_acct)
        - &near_string_to_yocto(&matcher2_offer.to_string());
    assert_approx_considering_gas(
        &matcher2.view_account(&worker).await?.balance,
        &matcher2_bal_after_offer,
    );

    //ONEDAY: Move to assert_expected_commitments helper function and reduce duplication.
    let commitments_result: String = contract
        .call(&worker, "get_commitments")
        .args_json(json!({"recipient": &recipient.id()}))?
        .gas(GAS_FOR_ACCOUNT_CALLBACK.0)
        .transact()
        .await?
        .json()
        .unwrap();

    assert_eq!(
        commitments_result,
        json!({
            matcher1.id().to_string(): matcher1_offer,
            matcher2.id().to_string(): matcher2_offer
        })
        .to_string()
    );

    let _matcher1_rescind_result = matcher1
        .call(&worker, contract.id(), "rescind_matching_funds")
        .args_json(
            json!({"recipient": &recipient.id(), "requested_withdrawal_amount": matcher1_rescind}),
        )?
        .gas(GAS_FOR_ACCOUNT_CALLBACK.0 * 3) // ONEDAY: Figure out how much gas to put here.
        .transact()
        .await?;
    let matcher1_offer_after_rescind =
        &near_string_to_yocto(&matcher1_offer) - &near_string_to_yocto(&matcher1_rescind);
    let commitments_result2: String = contract
        .call(&worker, "get_commitments")
        .args_json(json!({"recipient": &recipient.id()}))?
        .gas(GAS_FOR_ACCOUNT_CALLBACK.0)
        .transact()
        .await?
        .json()
        .unwrap();

    assert_eq!(
        commitments_result2,
        json!({
            matcher1.id().to_string(): yocto_to_near_string(&matcher1_offer_after_rescind),
            matcher2.id().to_string(): matcher2_offer
        })
        .to_string()
    );

    // TODO: Write the rest of the test.

    Ok(())
}
