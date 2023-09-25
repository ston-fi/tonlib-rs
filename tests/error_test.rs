use tonlib::address::TonAddressParseError;

mod common;

#[test]
#[ignore]
fn test_error_output() {
    common::init_logging();

    log::error!(
        "{}",
        TonAddressParseError::new(
            "Invalid base64 address",
            "EQQLKJGBEolgn2nl1;1`ln4141jl4n1n421n24142oololl",
        )
    );
}
