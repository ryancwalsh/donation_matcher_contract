#[cfg(all(test, not(target_arch = "wasm32")))]
mod helpers_tests {
    use crate::generic::{near_string_to_yocto, yocto_to_near_string};

    #[test]
    fn test_yocto_to_near() {
        // let mut context = get_context(accounts(2));
        // testing_env!(context.build());

        assert_eq!(
            yocto_to_near_string(&3_193_264_587_249_763_651_824_729),
            "3.1932 Ⓝ"
        ); // https://docs.rs/near-helper/latest/near_helper/fn.yoctonear_to_near.html
        assert_eq!(
            yocto_to_near_string(&21_409_258_000_000_000_000_000),
            "0.0214 Ⓝ"
        ); // https://docs.rs/near-helper/latest/near_helper/fn.yoctonear_to_near.html
        assert_eq!(
            yocto_to_near_string(&10_000_000_000_000_000_000_000),
            "0.01 Ⓝ"
        );
        assert_eq!(
            yocto_to_near_string(&700_000_000_000_000_000_000),
            "0.0007 Ⓝ"
        );
    }

    #[test]
    fn test_near_string_to_yocto() {
        assert_eq!(
            near_string_to_yocto(&"3.997 Ⓝ".to_string()),
            3_997_000_000_000_000_000_000_000
        );
        assert_eq!(
            near_string_to_yocto(&"0.018 Ⓝ".to_string()),
            18_000_000_000_000_000_000_000
        );
    }
}
