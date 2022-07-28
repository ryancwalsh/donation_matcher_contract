# TODO

// TODO: Update this readme to be relevant for this Rust repo.

# Donations With Automatic Matching Up to a Maximum

See original project written in AssemblyScript at https://github.com/ryancwalsh/donation-matching

Someone (let's call them "Matcher") wants to pledge to match donations to a certain account (the "Recipient") and says "I'll match donations up to X amount." So he/she sends the max amount to the controlling contract (sort of like escrow) and earmarks those funds for Recipient.

For simplicity (to avoid complexity of expiration dates and cron jobs and whatever), any Matcher can at any time rescind any unclaimed funds. If the promised funds from that Matcher towards a specific Recipient become 0, the Matcher is removed (is no longer listed as a Matcher related to that Recipient).

Any other account (other Donors) can choose to donate to the Recipient account (via the controlling contract, which is this project).

On each donation:

1. Funds that the Donor deposited into this escrow contract get transferred immediately to the Recipient.
1. For each and every "Matcher" account currently associated with the Recipient, the following happens:
   1. This escrow contract will automatically transfer to the Recipient an amount (called "matchedAmount") that equals the minimum of the donor's donated amount and that Matcher's remaining commitment to this Recipient.
   1. The Matcher's commitment will be decreased by that "matchedAmount".

---

# Usage

1. clone this repo to a local folder
1. `cargo install raen`
1. // TODO
1. Read https://docs.near.org/docs/tools/near-cli#near-call and decide whether you want to use `--depositYocto` or `--deposit` in the steps below.

## For localnet (work in progress; consider using testnet below for now):

1. `export NEAR_ENV=local`
1. [how to deploy contract locally?] `near dev-deploy $(raen build --release -q)`

```
near create-account justatemporarylocalaccount.node0 --masterAccount node0 --initialBalance 1000 --keyPath ~/.near/localnet/node0/validator_key.json

near create-account recipient.justatemporarylocalaccount.node0 --masterAccount justatemporarylocalaccount.node0 --initialBalance 1

near create-account matcher1.justatemporarylocalaccount.node0 --masterAccount justatemporarylocalaccount.node0 --initialBalance 1

near create-account matcher2.justatemporarylocalaccount.node0 --masterAccount justatemporarylocalaccount.node0 --initialBalance 1

near create-account donor.justatemporarylocalaccount.node0 --masterAccount justatemporarylocalaccount.node0 --initialBalance 1

export PARENT=justatemporarylocalaccount.node0
export MATCHER1=matcher1.justatemporarylocalaccount.node0
export MATCHER2=matcher2.justatemporarylocalaccount.node0
export RECIPIENT=recipient.justatemporarylocalaccount.node0
export DONOR=donor.justatemporarylocalaccount.node0
```

## For testnet:

1. `export NEAR_ENV=testnet`
1. `near dev-deploy $(raen build --release -q)` or `cargo build --target wasm32-unknown-unknown --release && near dev-deploy target/wasm32-unknown-unknown/release/donation_matcher_contract.wasm`
1. Run something like `export CONTRACT=dev-1638053233399-4079004334`.
1. `near call $CONTRACT new --accountId $CONTRACT --gas=15000000000000`
1. You will need at least 3 other NEAR accounts: one to act as a recipient (such as a charity), one to act as a regular donor, and one to act as a "matcher" (someone who commits to match others' donations to a certain recipient).

   - If you don't already have 3 testnet accounts that you want to use, you can create one (to serve as Matcher) at https://wallet.testnet.near.org/. Then you can create RECIPIENT and DONOR accounts as [sub-accounts](https://docs.near.org/docs/tools/near-cli#near-create-account) of that one. E.g.:

   ```
   near create-account recipient_b.ryancwalsh.testnet --masterAccount ryancwalsh.testnet --initialBalance 1
   near create-account matcher1_b.ryancwalsh.testnet --masterAccount ryancwalsh.testnet --initialBalance 1
   near create-account matcher2_b.ryancwalsh.testnet --masterAccount ryancwalsh.testnet --initialBalance 1
   near create-account donor_b.ryancwalsh.testnet --masterAccount ryancwalsh.testnet --initialBalance 1
   ```

1. Call `export` commands to define RECIPIENT, MATCHER, and DONOR with the accountIds from the previous steps. E.g.:

   ```
   export PARENT=ryancwalsh.testnet
   export MATCHER1=matcher1_b.ryancwalsh.testnet
   export MATCHER2=matcher2_b.ryancwalsh.testnet
   export RECIPIENT=recipient_b.ryancwalsh.testnet
   export DONOR=donor_b.ryancwalsh.testnet
   ```

## Now try using the contract (on localnet or testnet):

```
near state $MATCHER1 |  sed -n "s/.*formattedAmount: '\([^\\]*\).*'/\1/p"
near state $MATCHER2 |  sed -n "s/.*formattedAmount: '\([^\\]*\).*'/\1/p"
near state $RECIPIENT |  sed -n "s/.*formattedAmount: '\([^\\]*\).*'/\1/p"
near state $DONOR |  sed -n "s/.*formattedAmount: '\([^\\]*\).*'/\1/p"
near call $CONTRACT offer_matching_funds "{\"recipient\": \"$RECIPIENT\"}" --accountId $MATCHER1 --deposit .3 --gas=15000000000000
near call $CONTRACT offer_matching_funds "{\"recipient\": \"$RECIPIENT\"}" --accountId $MATCHER2 --deposit .1 --gas=15000000000000
near view $CONTRACT get_commitments "{\"recipient\": \"$RECIPIENT\"}"
near state $MATCHER1 |  sed -n "s/.*formattedAmount: '\([^\\]*\).*'/\1/p"
near state $MATCHER2 |  sed -n "s/.*formattedAmount: '\([^\\]*\).*'/\1/p"
```

In the approximate expected amounts below (denoted with `~`), consider that the amounts will actually be slightly less because some funds will have been sacrificed as gas costs.

(The result should reflect the values from above, and the CLI/Explorer should now also show Matcher1's balance as ~0.7 and Matcher2's balance as ~0.9.)

```
near call $CONTRACT rescind_matching_funds "{\"recipient\": \"$RECIPIENT\", \"requested_withdrawal_amount\": \"0.02 Ⓝ\"}" --accountId $MATCHER1 --gas=90000000000000
near view $CONTRACT get_commitments "{\"recipient\": \"$RECIPIENT\"}"
near state $MATCHER1 |  sed -n "s/.*formattedAmount: '\([^\\]*\).*'/\1/p"
```

(Matcher1 should now only have 0.3 - 0.02 = 0.28 committed to this Recipient. The CLI/Explorer should now also show Matcher1's balance as ~0.72.)

```
near call $CONTRACT donate "{\"recipient\": \"$RECIPIENT\"}" --accountId $DONOR --deposit .1 --gas 300000000000000
near state $MATCHER1 |  sed -n "s/.*formattedAmount: '\([^\\]*\).*'/\1/p"
near state $MATCHER2 |  sed -n "s/.*formattedAmount: '\([^\\]*\).*'/\1/p"
near state $RECIPIENT |  sed -n "s/.*formattedAmount: '\([^\\]*\).*'/\1/p"
near state $DONOR |  sed -n "s/.*formattedAmount: '\([^\\]*\).*'/\1/p"
near view $CONTRACT get_commitments "{\"recipient\": \"$RECIPIENT\"}"
```

(Only Matcher1 should be committed to 0.298 - 0.1 = 0.198.)

(The CLI/Explorer should now show:
Recipient's balance as 1 + (0.1 \* 3) = 1.3
Matcher1's balance as still ~0.702.
Matcher2's balance as still ~0.9.
Donor's balance is ~0.9.)

```
near call $CONTRACT rescind_matching_funds "{\"recipient\": \"$RECIPIENT\", \"requested_withdrawal_amount\": \"999 Ⓝ\"}" --accountId $MATCHER1 --gas=90000000000000
near state $MATCHER1 |  sed -n "s/.*formattedAmount: '\([^\\]*\).*'/\1/p"
near view $CONTRACT get_commitments "{\"recipient\": \"$RECIPIENT\"}"
```

(The CLI/Explorer should now show Matcher1's balance as ~0.702 + 0.198 = ~0.9 and get_commitments as empty.)

Optionally nuke the match relationships if they weren't already emptied: `near call $CONTRACT delete_all_matches_associated_with_recipient "{\"recipient\": \"$RECIPIENT\"}" --accountId $CONTRACT --gas=15000000000000`

Optionally clean up accounts with:

```
near delete $DONOR $PARENT
near delete $RECIPIENT $PARENT
near delete $MATCHER1 $PARENT
near delete $MATCHER2 $PARENT
near delete $CONTRACT $PARENT
```

Or do recreate all in one line and dev-deploy and delete_all_matches_associated_with_recipient:

```
near delete $DONOR $PARENT && near delete $RECIPIENT $PARENT && near delete $MATCHER1 $PARENT && near delete $MATCHER2 $PARENT && near create-account recipient_b.ryancwalsh.testnet --masterAccount ryancwalsh.testnet --initialBalance 1 && near create-account matcher1_b.ryancwalsh.testnet --masterAccount ryancwalsh.testnet --initialBalance 1 && near create-account matcher2_b.ryancwalsh.testnet --masterAccount ryancwalsh.testnet --initialBalance 1 && near create-account donor_b.ryancwalsh.testnet --masterAccount ryancwalsh.testnet --initialBalance 1 && near dev-deploy $(raen build --release -q) && near call $CONTRACT delete_all_matches_associated_with_recipient "{\"recipient\": \"$RECIPIENT\"}" --accountId $CONTRACT --gas=15000000000000
```
