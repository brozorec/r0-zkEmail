use serde::{Deserialize, Serialize};

/// Single email with its DKIM public key for verification
#[derive(Debug, Serialize, Deserialize)]
pub struct Email {
    pub from_domain: String,
    pub raw_email: Vec<u8>,
    pub public_key_type: String,
    pub public_key: Vec<u8>,
}

/// Input for the zkVM guest: two emails for secure verification
/// Both emails are DKIM verified independently
#[derive(Debug, Serialize, Deserialize)]
pub struct EmailPair {
    /// Original sender's email containing PAYMENT DATA (DKIM verified)
    pub sender_email: Email,
    /// Receiver's reply email containing PASSKEY DATA (DKIM verified)
    pub receiver_email: Email,
}

/// Output from the zkVM guest matching JournalOutput in contracts
#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentReceipt {
    /// Receiver's passkey public key (65 bytes, uncompressed secp256r1)
    pub receiver_pub_key: Vec<u8>,
    /// Credential identifier associated with the receiver's passkey
    pub receiver_cred_id: Vec<u8>,
    /// Payment amount in stroops
    pub amount: i128,
    /// Sender's Stellar address
    pub sender: String,
    /// Unique nonce for replay protection
    pub nonce: i32,
    /// Whether both emails passed DKIM verification
    pub verified: bool,
}
