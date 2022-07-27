// https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html

pub mod generic {
    use near_sdk::{env, log, CryptoHash, PromiseResult};

    pub type FormattedNearString = String; // (commas, underscores, and 'Ⓝ' are acceptable and will be ignored)

    const YOCTO_FACTOR: u128 = u128::pow(10, 24); // https://nomicon.io/Economics/Economic
    pub const DEFAULT_DECIMAL_PLACES: u32 = 4;

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

    /// Helper function to convert yoctoNEAR to $NEAR with _ decimals of precision.
    pub(crate) fn yocto_to_near(amount_in_yocto: u128, decimal_places: u32) -> f64 {
        // TODO: Audit
        let precision_multiplier = u128::pow(10, decimal_places);
        let formatted_near = amount_in_yocto * precision_multiplier / YOCTO_FACTOR;
        let near = formatted_near / precision_multiplier;
        near as f64
    }

    // TODO: Remove
    /// Helper function to convert yoctoNEAR to $NEAR with 4 decimals of precision.
    pub(crate) fn old_yocto_to_near(yocto: u128) -> f64 {
        //10^20 yoctoNEAR (1 NEAR would be 10_000). This is to give a precision of 4 decimal places.
        let formatted_near = yocto / 100_000_000_000_000_000_000;
        let near = formatted_near as f64 / 10_000_f64;

        near
    }

    // TODO: Remove
    /// Helper function to convert yoctoNEAR to $NEAR with _ decimals of precision.
    pub(crate) fn old_yocto_to_near_string(yocto: u128) -> String {
        let numeric = old_yocto_to_near(yocto);
        // ONEDAY: Add underscores or commas as thousands separators
        numeric.to_string() + &" Ⓝ".to_string()
    }

    /// Helper function to convert yoctoNEAR to $NEAR with _ decimals of precision.
    pub(crate) fn yocto_to_near_string(yocto: u128) -> String {
        let numeric = yocto_to_near(yocto, DEFAULT_DECIMAL_PLACES);
        // ONEDAY: Add underscores or commas as thousands separators
        numeric.to_string() + &" Ⓝ".to_string()
    }

    /// Convert $NEAR to yoctoNEAR.
    pub(crate) fn near_string_to_yocto(near_string: FormattedNearString) -> u128 {
        let cleaned = near_string
            .replace(",", "")
            .replace("_", "")
            .replace("Ⓝ", "");
        // TODO: Audit
        let near: f64 = cleaned.parse().expect("Could not convert NEAR from string to yoctoNEAR integer. Please check the formatting of your string.");
        let yocto = near as u128 * YOCTO_FACTOR;
        near_sdk::log!(
            "near_string_to_yocto converted {} (FormattedNear) to {} (yoctoNEAR)",
            near_string,
            yocto
        );
        yocto
    }
}
