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

fn assert_approx_considering_gas(amount1: &Balance, amount2str: &str) {
    const TOLERANCE: &str = &"0.0059 Ⓝ";
    let tolerance: Balance = near_string_to_yocto(&TOLERANCE.to_string());
    let amount2 = &near_string_to_yocto(&amount2str.to_string());
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

    let recipient = create_subaccount(&worker, &parent_account, "recipient", "1 Ⓝ").await?;

    assert_eq!(
        yocto_to_near_string(&recipient.view_account(&worker).await?.balance),
        "1 Ⓝ".to_string()
    );

    let matcher1 = create_subaccount(&worker, &parent_account, "matcher1", "1 Ⓝ").await?;

    let _matcher1_offer_result = matcher1
        .call(&worker, contract.id(), "offer_matching_funds")
        .args_json(json!({"recipient": &recipient.id()}))?
        .gas(GAS_FOR_ACCOUNT_CALLBACK.0)
        .deposit(near_string_to_yocto(&"0.3".to_string()))
        .transact()
        .await?;

    assert_eq!(
        yocto_to_near_string(&recipient.view_account(&worker).await?.balance),
        "1 Ⓝ".to_string()
    ); // The recipient hasn't received any donation yet.
    assert_approx_considering_gas(&matcher1.view_account(&worker).await?.balance, "0.7 Ⓝ");

    // TODO: Write the rest of the test.

    Ok(())
}
