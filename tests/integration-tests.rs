// https://github.com/near/workspaces-rs/blob/8f12f3dc3b0251ac3f44ddf6ab6fc63003579139/workspaces/tests/create_account.rs

#![recursion_limit = "256"]
use test_log::test;
use workspaces::prelude::*;

// TODO: Replace this test with real tests for this project.
#[test(tokio::test)]
async fn test_subaccount_creation() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let account = worker.dev_create_account().await?;

    let sub = account
        .create_subaccount(&worker, "subaccount")
        .transact()
        .await?
        .into_result()?;

    let expect_id = format!("subaccount.{}", account.id());
    let actual_id = sub.id().to_string();

    assert_eq!(actual_id, expect_id);

    Ok(())
}
