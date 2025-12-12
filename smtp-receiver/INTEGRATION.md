# Integration Architecture

This document explains how the SMTP receiver integrates with the existing r0-zkEmail components.

## Component Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        r0-zkEmail Workspace                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐      ┌──────────────┐      ┌──────────────┐   │
│  │    core/     │      │   methods/   │      │  extractor/  │   │
│  │              │      │              │      │              │   │
│  │ Email struct │◄─────┤ DKIM_VERIFY  │      │ Groth16      │   │
│  │ DKIMOutput   │      │ Guest code   │      │ Extractor    │   │
│  └──────────────┘      └──────────────┘      └──────────────┘   │
│         ▲                      ▲                      ▲         │
│         │                      │                      │         │
│         │                      │                      │         │
│  ┌──────┴──────────────────────┴──────────────────────┘         │
│  │                                                              │
│  │              ┌──────────────────────┐                        │
│  │              │      host/           │                        │
│  │              │                      │                        │
│  │              │  EmailVerifier       │                        │
│  │              │  - verify_email()    │                        │
│  │              │  - DKIM validation   │                        │
│  │              │  - ZK proof gen      │                        │
│  │              └──────────┬───────────┘                        │
│  │                         │                                    │
│  │                         │ Uses as library                    │
│  │                         │                                    │
│  │              ┌──────────▼───────────┐                        │
│  │              │  smtp-receiver/      │                        │
│  │              │                      │                        │
│  │              │  SmtpServer          │                        │
│  │              │  EmailHandler        │                        │
│  │              │  Config              │                        │
│  │              └──────────────────────┘                        │
│  │                         ▲                                    │
│  └─────────────────────────┘                                    │
│                            │                                    │
└────────────────────────────┼────────────────────────────────────┘
                             │
                    ┌────────▼────────┐
                    │  Incoming Email │
                    │   (SMTP Port)   │
                    └─────────────────┘
```

## Data Flow

### 1. Email Reception

```
Internet → SMTP Port 25/587/2525
           ↓
    SmtpServer (smtp_server.rs)
           ↓
    SmtpHandler::data_end()
           ↓
    Spawn async task
```

### 2. Email Processing

```
EmailHandler::handle_email()
    │
    ├─→ Save raw .eml file
    │   Location: received_emails/TIMESTAMP_FROM.eml
    │
    ├─→ Extract sender domain
    │   Example: "user@gmail.com" → "gmail.com"
    │
    ├─→ Check allowed_domains filter
    │   Skip if not in whitelist
    │
    └─→ Call verify_email_async()
```

### 3. DKIM Verification (via host crate)

```
EmailHandler::verify_email_async()
    │
    └─→ EmailVerifier::verify_email()
            │
            ├─→ Parse email (mailparse)
            │
            ├─→ Extract DKIM-Signature headers
            │
            ├─→ Retrieve public key from DNS
            │   Query: selector._domainkey.domain.com
            │
            ├─→ Verify DKIM signature (cfdkim)
            │
            └─→ Generate ZK proof
                    │
                    ├─→ Build ExecutorEnv
                    │
                    ├─→ Run guest code (DKIM_VERIFY_ELF)
                    │
                    ├─→ Get receipt with journal
                    │
                    ├─→ Compress to Succinct
                    │
                    └─→ Convert to Groth16 (if not dev mode)
```

### 4. Result Storage

```
Save proof data to JSON
    │
    └─→ proofs/TIMESTAMP_FROM.json
        {
          "timestamp": "...",
          "from_domain": "gmail.com",
          "filename": "...",
          "output": {
            "from_domain_hash": "...",
            "public_key_hash": "...",
            "verified": true,
            "hash_found": false
          }
        }
```

## Code Integration Points

### 1. Host Crate Refactoring

**Before:**
- `host/src/main.rs` - Monolithic binary with all logic

**After:**
- `host/src/lib.rs` - Library exposing `EmailVerifier`
- `host/src/main.rs` - CLI binary using the library

**Key Changes:**

```rust
// host/src/lib.rs
pub struct EmailVerifier {
    prover: Box<dyn Prover>,
    resolver: cfdkim::dns::Resolver,
}

impl EmailVerifier {
    pub async fn new() -> Result<Self> { ... }
    
    pub async fn verify_email(
        &self,
        from_domain: &str,
        raw_email: &str,
        target_hash: Option<String>,
    ) -> Result<DKIMOutput> { ... }
}
```

### 2. SMTP Receiver Integration

**Dependencies:**

```toml
[dependencies]
host = { path = "../host" }
zkemail-core = { path = "../core" }
```

**Usage:**

```rust
// smtp-receiver/src/email_handler.rs
use host::EmailVerifier;
use zkemail_core::DKIMOutput;

pub struct EmailHandler {
    verifier: EmailVerifier,
}

impl EmailHandler {
    pub async fn new() -> Result<Self> {
        let verifier = EmailVerifier::new().await?;
        Ok(Self { verifier })
    }
    
    pub async fn handle_email(&self, raw_email: &str) -> Result<()> {
        let output = self.verifier
            .verify_email(domain, raw_email, None)
            .await?;
        // Process output...
    }
}
```

## Shared Dependencies

### Core Types (zkemail-core)

```rust
// core/src/lib.rs
pub struct Email {
    pub from_domain: String,
    pub raw_email: Vec<u8>,
    pub public_key_type: String,
    pub public_key: Vec<u8>,
    pub target_hash: Option<String>,
}

pub struct DKIMOutput {
    pub from_domain_hash: Vec<u8>,
    pub public_key_hash: Vec<u8>,
    pub verified: bool,
    pub hash_found: bool,
}
```

Both `host` and `smtp-receiver` use these types.

### Methods (ZK Guest Code)

```rust
// methods/guest/src/main.rs
// Guest code that runs in RISC Zero zkVM
// Verifies DKIM signature and produces proof
```

Used by `EmailVerifier` in the host crate.

## Configuration Flow

### SMTP Receiver Config

```toml
# smtp-receiver/config.toml
[smtp]
port = 2525
allowed_domains = ["gmail.com"]

[processing]
auto_verify = true
```

### Environment Variables

```bash
# Shared by both host and smtp-receiver
RISC0_DEV_MODE=1    # Fast fake proofs for testing
RISC0_DEV_MODE=0    # Real proofs (requires Docker)
RUST_LOG=info       # Logging level
```

## File System Layout

```
r0-zkEmail/
├── core/                    # Shared types
│   └── src/lib.rs
│
├── methods/                 # ZK guest code
│   ├── guest/src/main.rs
│   └── src/lib.rs
│
├── host/                    # DKIM verifier (library + binary)
│   ├── src/
│   │   ├── lib.rs          # EmailVerifier (NEW)
│   │   └── main.rs         # CLI binary (refactored)
│   └── Cargo.toml
│
├── extractor/               # Groth16 proof extractor
│   └── src/main.rs
│
└── smtp-receiver/           # SMTP server (NEW)
    ├── src/
    │   ├── main.rs         # Entry point
    │   ├── smtp_server.rs  # SMTP protocol handler
    │   ├── email_handler.rs # Email processing
    │   └── config.rs       # Configuration
    ├── Cargo.toml
    ├── README.md
    ├── DEPLOYMENT.md
    └── config.example.toml
```

## Async Architecture

### Tokio Runtime

Both `host` and `smtp-receiver` use Tokio for async operations:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // ...
}
```

### Non-Blocking Email Processing

```rust
// smtp-receiver/src/smtp_server.rs
fn data_end(&mut self, ..., data: Vec<u8>) -> Response {
    // Spawn async task to avoid blocking SMTP server
    tokio::spawn(async move {
        handler.handle_email(&raw_email, &from, &to).await
    });
    
    // Return immediately to SMTP client
    mailin_embedded::response::OK
}
```

This ensures the SMTP server can accept new connections while processing emails in the background.

## Error Handling

### Graceful Degradation

```rust
// smtp-receiver/src/email_handler.rs
match self.verify_email_async(...).await {
    Ok(_) => info!("Verification successful"),
    Err(e) => {
        // Log error but don't crash
        error!("Verification failed: {}", e);
        // Email is still saved to disk
    }
}
```

### Logging Strategy

- **SMTP events**: Connection, HELO, MAIL FROM, RCPT TO, DATA
- **Email processing**: Save, parse, domain extraction
- **DKIM verification**: DNS queries, signature validation
- **ZK proof generation**: Proof generation, compression, Groth16 conversion

## Performance Considerations

### ZK Proof Generation

- **Dev Mode**: ~1-5 seconds (fake proofs)
- **Production Mode**: ~5-30 minutes (real proofs with Groth16)

### Async Processing

Email verification runs in background task:
- SMTP server remains responsive
- Multiple emails can be processed concurrently
- No blocking on proof generation

### Resource Usage

- **CPU**: High during proof generation
- **Memory**: 2-4GB per proof generation
- **Disk**: ~10KB per email, ~1MB per proof

## Testing Integration

### Unit Tests

```bash
# Test host library
cd host
cargo test

# Test smtp-receiver
cd smtp-receiver
cargo test
```

### Integration Tests

```bash
# Terminal 1: Start SMTP receiver
cd smtp-receiver
RISC0_DEV_MODE=1 RUST_LOG=info cargo run

# Terminal 2: Send test email
./test-local.sh

# Check results
ls -lh received_emails/
ls -lh proofs/
```

### End-to-End Test

```bash
# Forward a real email from Gmail to your SMTP server
# Check that:
# 1. Email is saved to received_emails/
# 2. DKIM verification succeeds
# 3. Proof is generated and saved to proofs/
# 4. Proof data is valid JSON with correct structure
```

## Future Enhancements

### Potential Improvements

1. **TLS Support**: Encrypt SMTP connections
2. **Rate Limiting**: Prevent spam floods
3. **Webhook Integration**: Notify external services on email receipt
4. **Database Storage**: Store emails and proofs in PostgreSQL
5. **Web Dashboard**: View received emails and proofs
6. **Email Forwarding**: Forward verified emails to another address
7. **Batch Processing**: Process multiple emails in parallel
8. **Proof Caching**: Cache proofs for duplicate emails

### Extension Points

```rust
// smtp-receiver/src/email_handler.rs
pub trait EmailProcessor {
    async fn process(&self, email: &Email) -> Result<()>;
}

// Implement custom processors
struct WebhookProcessor { ... }
struct DatabaseProcessor { ... }
struct ForwardingProcessor { ... }
```

## Troubleshooting Integration Issues

### Build Errors

```bash
# Clean build
cargo clean
cargo build --release

# Check dependencies
cargo tree | grep host
cargo tree | grep zkemail-core
```

### Runtime Errors

```bash
# Enable debug logging
RUST_LOG=debug cargo run --bin smtp-receiver

# Check host library is accessible
cargo run --bin host -- --help
```

### Verification Failures

```bash
# Test host library directly
cargo run --bin host -- gmail.com test.eml

# Compare with SMTP receiver results
cat proofs/latest.json
```
