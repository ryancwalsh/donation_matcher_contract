// https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html

pub mod generic {
    use near_sdk::{env, log, CryptoHash, PromiseResult};

    /// Aux functions to interact with the validator
    // https://docs.near.org/develop/contracts/crosscontract#snippet-sending-information
    pub fn did_promise_succeed() -> bool {
        if env::promise_results_count() != 1 {
            log!("Expected a result on the callback");
            return false;
        }

        match env::promise_result(0) {
            PromiseResult::Successful(_) => true,
            _ => false,
        }
    }

    /// Used to generate a unique prefix in our storage collections (this is to avoid data collisions; see https://stackoverflow.com/questions/65248816/why-should-i-hash-keys-in-the-nearprotocol-unorderedmap)
    pub(crate) fn hash_account_id(account_id: &String) -> CryptoHash {
        env::sha256_array(account_id.as_bytes())
    }

    /// Helper function to convert yoctoNEAR to $NEAR with 4 decimals of precision.
    pub(crate) fn yocto_to_near(yocto: u128) -> f64 {
        //10^20 yoctoNEAR (1 NEAR would be 10_000). This is to give a precision of 4 decimal places.
        let formatted_near = yocto / 100_000_000_000_000_000_000;
        let near = formatted_near as f64 / 10_000_f64;

        near
    }

    /// Helper function to convert yoctoNEAR to $NEAR with 4 decimals of precision.
    pub(crate) fn yocto_to_near_string(yocto: u128) -> String {
        let numeric = yocto_to_near(yocto);
        numeric.to_string() + &" Ⓝ".to_string()
    }
}
