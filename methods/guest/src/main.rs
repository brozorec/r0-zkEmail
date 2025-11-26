use cfdkim::{verify_email_with_key, DkimPublicKey};
use email_parser::{
    extract_email_address, extract_passkey, extract_payment_data, get_email_body, get_header,
};
use mailparse::parse_mail;
use risc0_zkvm::guest::env;
use slog::{o, Discard, Logger};
use zkemail_core::{Email, EmailPair, PaymentReceipt};

fn main() {
    let input: Vec<u8> = env::read_frame();
    let input: EmailPair = postcard::from_bytes(&input).unwrap();

    let logger = Logger::root(Discard, o!());

    // Verify DKIM signatures for BOTH emails
    let sender_verified = verify_email(&logger, &input.sender_email);
    let receiver_verified = verify_email(&logger, &input.receiver_email);

    // Parse both emails to extract headers
    let sender_parsed =
        parse_mail(&input.sender_email.raw_email).expect("Failed to parse sender email");
    let receiver_parsed =
        parse_mail(&input.receiver_email.raw_email).expect("Failed to parse receiver email");

    // Get From/To headers
    let sender_from = get_header(&sender_parsed, "From").expect("Missing From in sender email");
    let sender_to = get_header(&sender_parsed, "To").expect("Missing To in sender email");
    let receiver_from =
        get_header(&receiver_parsed, "From").expect("Missing From in receiver email");
    let receiver_to = get_header(&receiver_parsed, "To").expect("Missing To in receiver email");

    // Extract email addresses
    let sender_addr = extract_email_address(&sender_from);
    let receiver_addr = extract_email_address(&receiver_from);

    // Validate email chain:
    // 1. Sender's email must have been sent TO the receiver
    assert!(
        sender_to.contains(&receiver_addr),
        "Sender's email must be addressed to the receiver"
    );
    // 2. Receiver's reply must be addressed TO the original sender
    assert!(
        receiver_to.contains(&sender_addr),
        "Receiver's reply must be addressed to the original sender"
    );

    // Extract payment data from sender's email (DKIM verified)
    let sender_body = get_email_body(&input.sender_email.raw_email);
    let (stellar_sender, amount, nonce) = extract_payment_data(&sender_body)
        .expect("Failed to extract payment data from sender email");

    // Extract passkey from receiver's email (DKIM verified)
    let receiver_body = get_email_body(&input.receiver_email.raw_email);
    let receiver_passkey =
        extract_passkey(&receiver_body).expect("Failed to extract passkey from receiver email");

    let output = PaymentReceipt {
        receiver_passkey,
        amount,
        sender: stellar_sender,
        nonce,
        verified: sender_verified && receiver_verified,
    };

    env::commit(&output);
}

/// Verify DKIM signature of an email
fn verify_email(logger: &slog::Logger, email: &Email) -> bool {
    let parsed_email = match parse_mail(&email.raw_email) {
        Ok(e) => e,
        Err(_) => return false,
    };

    let public_key = match DkimPublicKey::try_from_bytes(&email.public_key, &email.public_key_type)
    {
        Ok(k) => k,
        Err(_) => return false,
    };

    match verify_email_with_key(logger, &email.from_domain, &parsed_email, public_key) {
        Ok(result) => result.with_detail().starts_with("pass"),
        Err(_) => false,
    }
}
