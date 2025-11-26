use crate::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::Env;

// Test data from the provided mock values
//const SEAL_HEX: &str = "73c457ba113f4d90b599377a6622efc6b6058ae339c1a5363134228a8268976229ce19ae2564d227ba2fb1f5156603d7e0843f7ccbcb9e2c97abd98a982eec705c464c1118ffcecab2035eaf4dda926c3eddb7809d8b5a5d05ad673fea903029b8536e610ddd43c62f3bc7e0941d7cb2297d3d5b8593b120077ac932d3388da7fb3b101b21bbfa5304ffa102b76425dbf69f8258c3251b65bb8cb8edaa93153d7b563c6c09a80ddbda0cdae890d13179733a8cca3dcbea9d6bfa006eec0e51355ba5c7ff1f2e4b62ed322deda1bf642195f07f9872e1c4359a64b7832ee19c1f792e71b72a387b3adf5aa4c3b3c4b38f56ea4262c5caf19252a39efeff914bc7a114da1c";
//const IMAGE_ID_HEX: &str = "d8cbfaac3f4775224bd8d0ab67ec307cd0c400e631157312de4dff7168fa93ec";
const JOURNAL_HEX: &str = "4100000004000000f900000047000000be0000000b0000003d000000f0000000ae00000082000000c2000000fa00000088000000b300000071000000ee0000009e000000ff00000017000000ca000000950000009d0000005c00000088000000930000002c000000a70000001d000000720000007600000019000000850000000f0000004500000042000000f8000000f10000001500000038000000910000000c0000003e0000005d000000fe000000110000006200000064000000ad000000ca0000005000000025000000a40000009f000000d60000000700000047000000b1000000ba000000820000000f00000090000000e2000000d90000006b000000a10000004b00000000ca9a3b0000000000000000000000003800000043414d535246424f584942435632444f4634525a433337463245524258415135574e33524e49524c56444a4d5a534c5033444e49354454332536edff01000000";

fn hex_to_bytes(e: &Env, hex: &str) -> Bytes {
    let mut bytes = Bytes::new(e);
    for i in (0..hex.len()).step_by(2) {
        let byte = u8::from_str_radix(&hex[i..i + 2], 16).unwrap();
        bytes.push_back(byte);
    }
    bytes
}

#[test]
fn test_decode_journal() {
    let e = Env::default();
    let journal = hex_to_bytes(&e, JOURNAL_HEX);

    let output = decode_journal(&e, &journal);

    // Verify passkey is 65 bytes (starts with 0x04 for uncompressed)
    assert_eq!(output.receiver_passkey.len(), 65);
    // First byte should be 0x04 (uncompressed key prefix)
    let mut passkey_arr = [0u8; 65];
    output.receiver_passkey.copy_into_slice(&mut passkey_arr);
    assert_eq!(passkey_arr[0], 0x04);

    // Verify amount: 0x3b9aca00 = 1_000_000_000
    assert_eq!(output.amount, 1_000_000_000);

    // Verify sender address (56 chars)
    let expected_sender = String::from_bytes(
        &e,
        b"CAMSRFBOXIBCV2DOF4RZC37F2ERBXAQ5WN3RNIRLVDJMZSLP3DNI5DT3",
    );
    assert_eq!(output.sender, expected_sender);

    // Verify nonce: 0xffed3625 in little-endian = -1231323
    assert_eq!(output.nonce, -1231323);

    // Verify verified flag
    assert!(output.verified);
}

#[test]
fn test_constructor_stores_addresses() {
    let e = Env::default();

    let risc0_verifier = Address::generate(&e);
    let passkey_verifier = Address::generate(&e);
    let token = Address::generate(&e);

    // Constructor args are passed during registration
    let contract_id = e.register(
        EmailPaymentGateway,
        (
            risc0_verifier.clone(),
            passkey_verifier.clone(),
            token.clone(),
        ),
    );
    let client = EmailPaymentGatewayClient::new(&e, &contract_id);

    assert_eq!(client.get_risc0_verifier(), risc0_verifier);
    assert_eq!(client.get_passkey_verifier(), passkey_verifier);
    assert_eq!(client.get_token(), token);
}

#[test]
fn test_read_u32_le() {
    let e = Env::default();
    // 0x41000000 in little-endian = 65
    let bytes = hex_to_bytes(&e, "41000000");
    assert_eq!(read_u32_le(&bytes, 0), 65);
}

#[test]
fn test_read_i128_le() {
    let e = Env::default();
    // 1_000_000_000 as i128 little-endian (16 bytes)
    let bytes = hex_to_bytes(&e, "00ca9a3b00000000000000000000000000");
    assert_eq!(read_i128_le(&bytes, 0), 1_000_000_000);
}

#[test]
fn test_read_i32_le() {
    let e = Env::default();
    // -1231323 as i32 little-endian
    let bytes = hex_to_bytes(&e, "2536edff");
    assert_eq!(read_i32_le(&bytes, 0), -1231323);
}

#[test]
fn test_read_string() {
    let e = Env::default();
    // "CB" as raw bytes
    let hex = "4342";
    let bytes = hex_to_bytes(&e, hex);
    let result = read_string(&e, &bytes, 0, 2);
    let expected = String::from_bytes(&e, b"CB");
    assert_eq!(result, expected);
}

#[test]
#[should_panic(expected = "receiver_passkey must be 65 bytes")]
fn test_decode_journal_invalid_passkey_length() {
    let e = Env::default();
    // Journal with wrong passkey length (32 instead of 65)
    let mut bad_journal = Bytes::new(&e);
    // Length = 32 (wrong)
    bad_journal.push_back(0x20);
    bad_journal.push_back(0x00);
    bad_journal.push_back(0x00);
    bad_journal.push_back(0x00);

    decode_journal(&e, &bad_journal);
}
