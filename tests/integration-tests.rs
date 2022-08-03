// https://github.com/near/workspaces-rs/blob/8f12f3dc3b0251ac3f44ddf6ab6fc63003579139/workspaces/tests/create_account.rs

#![recursion_limit = "256"]

use anyhow::Error;
use donation_matcher_contract::{generic::near_string_to_yocto, GAS_FOR_ACCOUNT_CALLBACK};
use near_sdk::serde_json::json;
use test_log::test;
use workspaces::{network::Sandbox, prelude::*, Account, Worker};

async fn create_subaccount(
    worker: &Worker<Sandbox>,
    parent_account: &Account,
    name: &str,
    amount: &str,
) -> Result<Account, Error> {
    let subaccount = parent_account
        .create_subaccount(&worker, name)
        .transact()
        .await?
        .into_result()?;

    parent_account.transfer_near(
        &worker,
        &subaccount.id(),
        near_string_to_yocto(amount.to_string()),
    );
    Ok(subaccount)
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

    let recipient = create_subaccount(&worker, &parent_account, "recipient", "1").await?;

    let matcher1 = create_subaccount(&worker, &parent_account, "matcher1", "1").await?;

    let _matcher1_offer_result = contract
        .call(&worker, "offer_matching_funds")
        .args_json(json!({"recipient": &recipient.id()}))?
        .gas(GAS_FOR_ACCOUNT_CALLBACK.0)
        .deposit(near_string_to_yocto("0.3".to_string()))
        .transact()
        .await?;

    assert_eq!(
        matcher1.view_account(&worker).await?.balance,
        near_string_to_yocto("0.7".to_string())
    );
    assert_eq!(
        recipient.view_account(&worker).await?.balance,
        near_string_to_yocto("1".to_string())
    ); // The recipient hasn't received any donation yet.

    // TODO: Write the rest of the test.

    Ok(())
}
