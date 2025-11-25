//! Test binary to verify email parsing logic outside the zkVM.
//! Usage: cargo run --bin test_parser -- <sender.eml> <receiver.eml>
//!
//! Both emails are verified independently:
//! - sender.eml: Contains PAYMENT DATA (SENDER, AMOUNT, NONCE)
//! - receiver.eml: Contains PASSKEY DATA (PUBLIC_KEY)

use dkim_verify::{extract_email_address, extract_passkey, extract_payment_data, get_email_body, get_header};
use mailparse::parse_mail;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 3 {
        eprintln!("Usage: {} <sender.eml> <receiver.eml>", args[0]);
        eprintln!("  sender.eml   - Original email with PAYMENT DATA block");
        eprintln!("  receiver.eml - Reply email with PASSKEY DATA block");
        std::process::exit(1);
    }

    let sender_path = &args[1];
    let receiver_path = &args[2];

    let sender_raw = fs::read(sender_path).expect("Failed to read sender email");
    let receiver_raw = fs::read(receiver_path).expect("Failed to read receiver email");

    println!("=== Sender's Email (Payment Request) ===");
    let (sender_addr, sender_to) = test_sender_email(&sender_raw);

    println!("\n=== Receiver's Email (Passkey Reply) ===");
    let (receiver_addr, receiver_to) = test_receiver_email(&receiver_raw);

    println!("\n=== Email Chain Validation ===");
    validate_chain(&sender_addr, &sender_to, &receiver_addr, &receiver_to);
}

fn test_sender_email(raw: &[u8]) -> (String, String) {
    let parsed = parse_mail(raw).expect("Failed to parse sender email");
    
    let from = get_header(&parsed, "From").unwrap_or_default();
    let to = get_header(&parsed, "To").unwrap_or_default();
    let subject = get_header(&parsed, "Subject").unwrap_or_default();
    
    println!("From: {}", from);
    println!("To: {}", to);
    println!("Subject: {}", subject);

    let body = get_email_body(raw);

    println!("\n--- Payment Data ---");
    match extract_payment_data(&body) {
        Some((sender, amount, nonce)) => {
            println!("SENDER (Stellar address): {}", sender);
            println!("AMOUNT: {}", amount);
            println!("NONCE: {}", nonce);
        }
        None => {
            println!("ERROR: Could not extract payment data!");
            println!("Looking for block:");
            println!("  ---BEGIN PAYMENT DATA---");
            println!("  SENDER: <stellar_address>");
            println!("  AMOUNT: <number>");
            println!("  NONCE: <number>");
            println!("  ---END PAYMENT DATA---");
        }
    }

    (extract_email_address(&from), to)
}

fn test_receiver_email(raw: &[u8]) -> (String, String) {
    let parsed = parse_mail(raw).expect("Failed to parse receiver email");
    
    let from = get_header(&parsed, "From").unwrap_or_default();
    let to = get_header(&parsed, "To").unwrap_or_default();
    let subject = get_header(&parsed, "Subject").unwrap_or_default();
    
    println!("From: {}", from);
    println!("To: {}", to);
    println!("Subject: {}", subject);

    let body = get_email_body(raw);

    println!("\n--- Passkey Data ---");
    match extract_passkey(&body) {
        Some(passkey) => {
            println!("Passkey length: {} bytes", passkey.len());
            println!("Passkey (hex): {}", hex_encode(&passkey));
            if passkey.len() == 65 && passkey[0] == 0x04 {
                println!("Valid uncompressed secp256r1 public key format");
            } else {
                println!("WARNING: Expected 65-byte uncompressed key starting with 0x04");
            }
        }
        None => {
            println!("ERROR: Could not extract passkey!");
            println!("Looking for block:");
            println!("  ---BEGIN PASSKEY DATA---");
            println!("  PUBLIC_KEY: <hex>");
            println!("  ---END PASSKEY DATA---");
        }
    }

    (extract_email_address(&from), to)
}

fn validate_chain(sender_addr: &str, sender_to: &str, receiver_addr: &str, receiver_to: &str) {
    println!("Sender address: {}", sender_addr);
    println!("Sender's To: {}", sender_to);
    println!("Receiver address: {}", receiver_addr);
    println!("Receiver's To: {}", receiver_to);

    // Check 1: Sender's email was sent TO the receiver
    if sender_to.contains(receiver_addr) {
        println!("OK: Sender's email was addressed to receiver");
    } else {
        println!("FAIL: Sender's To does not contain receiver's address");
    }

    // Check 2: Receiver's reply is addressed TO the sender
    if receiver_to.contains(sender_addr) {
        println!("OK: Receiver's reply is addressed to sender");
    } else {
        println!("FAIL: Receiver's To does not contain sender's address");
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
