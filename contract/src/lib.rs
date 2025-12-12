#![no_std]

use risc0_interface::RiscZeroVerifierClient;
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, map, panic_with_error, token::TokenClient,
    vec, Address, Bytes, BytesN, Env, Map, String, Val,
};
use stellar_accounts::smart_account::Signer;

const WASM: &[u8] = include_bytes!("../smart_account.wasm");

#[contracterror]
enum Errors {
    UsedNonce = 1,
    EmailUnverified = 2,
}

#[contracttype]
enum DataKey {
    Risc0Verifier,
    PasskeyVerifier,
    Token,
    Nonce(Address, i32), // tracks nonces per sender
}

/// Decoded verification output from the zkVM journal.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JournalOutput {
    pub receiver_pub_key: Bytes,
    pub receiver_cred_id: Bytes,
    pub amount: i128,
    pub sender: String,
    pub nonce: i32,
    pub verified: bool,
}

/// Decodes a RISC0 journal into JournalOutput.
///
/// RISC0 journal uses word-aligned serialization where each byte/char is stored as u32.
/// Format:
/// - 4 bytes: length of receiver_pub_key (should be 65)
/// - 65 * 4 bytes: receiver_pub_key (each byte as u32)
/// - 4 bytes: length of receiver_cred_id (variable)
/// - N * 4 bytes: receiver_cred_id (each byte as u32)
/// - 16 bytes: amount (i128, little-endian)
/// - 4 bytes: length of sender string
/// - N * 4 bytes: sender string (each char as u32)
/// - 4 bytes: nonce (i32, little-endian)
/// - 4 bytes: verified (bool as u32)
pub(crate) fn decode_journal(e: &Env, journal: &Bytes) -> JournalOutput {
    let mut offset: u32 = 0;

    // Read receiver_pub_key length (4 bytes, little-endian)
    let pubkey_len = read_u32_le(journal, offset);
    assert!(pubkey_len == 65, "receiver_pub_key must be 65 bytes");
    offset += 4;

    // Read receiver_pub_key (65 bytes, each stored as u32)
    let receiver_pub_key = read_bytes_words(e, journal, offset, pubkey_len);
    offset += 65 * 4;

    // Read receiver_cred_id length and bytes
    let credential_len = read_u32_le(journal, offset);
    offset += 4;
    let receiver_cred_id = read_bytes_words(e, journal, offset, credential_len);
    offset += credential_len * 4;

    // Read amount (16 bytes, i128 little-endian)
    let amount = read_i128_le(journal, offset);
    offset += 16;

    // Read sender string length (4 bytes)
    let sender_len = read_u32_le(journal, offset);
    offset += 4;

    // Read sender string (raw bytes, NOT word-aligned)
    let sender = read_string(e, journal, offset, sender_len);
    offset += sender_len;

    // Read nonce (4 bytes, i32 little-endian)
    let nonce = read_i32_le(journal, offset);
    offset += 4;

    // Read verified (stored as u32)
    let verified = read_u32_le(journal, offset) != 0;

    JournalOutput {
        receiver_pub_key,
        receiver_cred_id,
        amount,
        sender,
        nonce,
        verified,
    }
}

pub(crate) fn read_u32_le(bytes: &Bytes, offset: u32) -> u32 {
    let b0 = bytes.get(offset).expect("missing byte") as u32;
    let b1 = bytes.get(offset + 1).expect("missing byte") as u32;
    let b2 = bytes.get(offset + 2).expect("missing byte") as u32;
    let b3 = bytes.get(offset + 3).expect("missing byte") as u32;
    b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
}

/// Read variable-length bytes stored as 4-byte words
pub(crate) fn read_bytes_words(e: &Env, bytes: &Bytes, offset: u32, len: u32) -> Bytes {
    let mut out = Bytes::new(e);
    for i in 0..len {
        let value = bytes.get(offset + i * 4).expect("missing byte");
        out.push_back(value);
    }
    out
}

pub(crate) fn read_i128_le(bytes: &Bytes, offset: u32) -> i128 {
    let mut arr = [0u8; 16];
    for i in 0..16 {
        arr[i as usize] = bytes.get(offset + i).expect("missing byte");
    }
    i128::from_le_bytes(arr)
}

pub(crate) fn read_i32_le(bytes: &Bytes, offset: u32) -> i32 {
    let b0 = bytes.get(offset).expect("missing byte") as u32;
    let b1 = bytes.get(offset + 1).expect("missing byte") as u32;
    let b2 = bytes.get(offset + 2).expect("missing byte") as u32;
    let b3 = bytes.get(offset + 3).expect("missing byte") as u32;
    (b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)) as i32
}

pub(crate) fn read_string(e: &Env, bytes: &Bytes, offset: u32, len: u32) -> String {
    let mut arr = [0u8; 56]; // Stellar addresses are 56 chars
    let actual_len = if len > 56 { 56 } else { len };
    for i in 0..actual_len {
        arr[i as usize] = bytes.get(offset + i).expect("missing byte");
    }
    String::from_bytes(e, &arr[..actual_len as usize])
}

#[contract]
pub struct EmailPaymentGateway;

#[contractimpl]
impl EmailPaymentGateway {
    pub fn __constructor(
        e: &Env,
        risc0_verifier: Address,
        passkey_verifier: Address,
        token: Address,
    ) {
        e.storage()
            .instance()
            .set(&DataKey::Risc0Verifier, &risc0_verifier);
        e.storage()
            .instance()
            .set(&DataKey::PasskeyVerifier, &passkey_verifier);
        e.storage().instance().set(&DataKey::Token, &token);
    }

    pub fn get_risc0_verifier(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::Risc0Verifier)
            .expect("verifier not set")
    }

    pub fn get_passkey_verifier(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::PasskeyVerifier)
            .expect("verifier not set")
    }

    pub fn get_token(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::Token)
            .expect("token not set")
    }

    pub fn verify_and_pay(e: &Env, seal: Bytes, image_id: BytesN<32>, journal: Bytes) {
        // Decode and return the journal contents
        let output = decode_journal(e, &journal);

        if !output.verified {
            panic_with_error!(e, Errors::EmailUnverified)
        }

        let sender = Address::from_string(&output.sender);
        let key_nonce = DataKey::Nonce(sender.clone(), output.nonce);
        if e.storage().persistent().has(&key_nonce) {
            panic_with_error!(e, Errors::UsedNonce)
        }
        e.storage().persistent().set(&key_nonce, &true);

        // Verify the proof with the RISC0 verifier
        let verifier_addr = Self::get_risc0_verifier(e);
        let client = RiscZeroVerifierClient::new(e, &verifier_addr);

        let journal_digest = e.crypto().sha256(&journal).into();
        // Panics if can't verify
        client.verify(&seal, &image_id, &journal_digest);

        // TODO: change salt
        let deployer = e.deployer().with_current_contract(journal_digest);
        let wasm_hash = e.deployer().upload_contract_wasm(WASM);

        // passkey
        // passkey verifier addr - from this storage
        // -> deploy smart account with passkey
        let mut passkey = output.receiver_pub_key;
        passkey.append(&output.receiver_cred_id);
        let passkey_verifier = Self::get_passkey_verifier(e);
        let signers = vec![e, Signer::External(passkey_verifier, passkey)];
        let policies: Map<Address, Val> = map![e];
        let receiver = deployer.deploy_v2(wasm_hash, (signers, policies));

        // usdc contract - from this storage
        // sender account
        // amount
        // -> .transfer_from()
        let token_addr = Self::get_token(e);
        let token = TokenClient::new(e, &token_addr);
        token.transfer_from(
            &e.current_contract_address(),
            &sender,
            &receiver,
            &output.amount,
        );
    }
}

#[cfg(test)]
mod test;
