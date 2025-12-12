# SMTP Receiver for r0-zkEmail

A simple SMTP server that receives emails directly and automatically verifies DKIM signatures using zero-knowledge proofs.

## Features

- **Direct SMTP Reception**: Receives emails via SMTP protocol (no third-party services)
- **Automatic DKIM Verification**: Integrates with the existing r0-zkEmail host to verify DKIM signatures
- **Zero-Knowledge Proofs**: Generates ZK proofs for email authenticity
- **Configurable**: Simple TOML configuration for domains, storage, and processing options
- **Async Processing**: Non-blocking email verification using Tokio

## Quick Start

### 1. Generate Configuration

```bash
cargo run --bin smtp-receiver -- --generate-config config.toml
```

This creates a `config.toml` file with default settings:

```toml
[smtp]
bind_address = "0.0.0.0"
port = 2525
max_message_size = 52428800  # 50 MB
allowed_domains = []  # Empty = allow all

[storage]
email_dir = "./received_emails"
proof_dir = "./proofs"

[processing]
validate_email_format = true

[runpod]
# Required only when compiling without the `verify` feature
runpod_url = "https://nacgo2o3dv4i5h.api.runpod.ai/generate"
runpod_key = "RUNPOD_API_KEY"
retry_attempts = 3
retry_delay_decs = 10
```

### 2. Run the Server

```bash
# Development mode (fake proofs, faster)
RISC0_DEV_MODE=1 RUST_LOG=info cargo run --bin smtp-receiver -- config.toml

# Production mode (real proofs, requires Docker)
RISC0_DEV_MODE=0 RUST_LOG=info cargo run --bin smtp-receiver
```

### 3. Test with a Local Email

Send a test email using `swaks` or similar:

```bash
swaks --to test@yourdomain.com \
      --from sender@example.com \
      --server localhost:2525 \
      --body "Test email for DKIM verification"
```

## Configuration Options

### SMTP Settings

- **`bind_address`**: IP address to bind (use `0.0.0.0` for all interfaces)
- **`port`**: SMTP port (default: 2525 for testing, use 25 for production)
- **`max_message_size`**: Maximum email size in bytes
- **`allowed_domains`**: List of domains to process (empty = all domains)

### Storage Settings

- **`email_dir`**: Directory to save received `.eml` files
- **`proof_dir`**: Directory to save verification proofs (JSON format)

### Processing Settings

- **`validate_email_format`**: If true, only store emails that match the expected payment/passkey format

### Runpod Settings (non-`verify` builds)

When running a binary compiled **without** the `verify` feature, verification is delegated to a remote Runpod worker. These settings configure the HTTP client the SMTP receiver uses:

- **`runpod_url`**: HTTPS endpoint that accepts the verification payload
- **`runpod_key`**: Bearer token used for authentication
- **`retry_attempts`** *(optional, default `3`)*: Number of times to retry failed requests
- **`retry_delay_decs`** *(optional, default `10` = 1s)*: Delay between retries in deciseconds

If the `runpod` section is missing while running a non-verify build, matched email pairs will fail verification.

## Infrastructure Setup

### VM Configuration

#### 1. Open Firewall Ports

```bash
# For testing (non-privileged port)
sudo ufw allow 2525/tcp

# For production (standard SMTP port)
sudo ufw allow 25/tcp

# Alternative submission port
sudo ufw allow 587/tcp
```

#### 2. DNS Records (MX)

Add these DNS records to your domain:

```
your-domain.com.         IN  MX  10 mail.your-domain.com.
mail.your-domain.com.    IN  A   <YOUR_VM_IP>
```

#### 3. Reverse DNS (PTR Record)

Configure reverse DNS through your hosting provider:

```
<YOUR_VM_IP>  →  mail.your-domain.com
```

This is critical for email delivery reputation.

### Running on Port 25 (Production)

Port 25 requires root privileges. Use one of these approaches:

#### Option 1: Use `setcap` (Recommended)

```bash
# Build the binary
cargo build --release --bin smtp-receiver

# Grant capability to bind to privileged ports
sudo setcap 'cap_net_bind_service=+ep' target/release/smtp-receiver

# Run as non-root user
./target/release/smtp-receiver config.toml
```

#### Option 2: Use systemd with User Permissions

Create `/etc/systemd/system/smtp-receiver.service`:

```ini
[Unit]
Description=r0-zkEmail SMTP Receiver
After=network.target

[Service]
Type=simple
User=your-user
WorkingDirectory=/path/to/r0-zkEmail/smtp-receiver
Environment="RUST_LOG=info"
Environment="RISC0_DEV_MODE=1"
ExecStart=/path/to/r0-zkEmail/target/release/smtp-receiver /path/to/config.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable smtp-receiver
sudo systemctl start smtp-receiver
sudo systemctl status smtp-receiver
```

## Integration with Existing Components

### Architecture

```
Incoming Email (SMTP)
    ↓
smtp-receiver (this crate)
    ↓
email_handler.rs
    ↓
host::EmailVerifier (host/src/lib.rs)
    ↓
DKIM Verification + ZK Proof Generation
    ↓
Save .eml + proof.json
```

### Data Flow

1. **Email Reception**: SMTP server receives raw email
2. **Storage**: Saves `.eml` file to `received_emails/`
3. **Domain Extraction**: Extracts sender domain (e.g., `gmail.com`)
4. **DKIM Verification**: Calls `EmailVerifier::verify_email()` from host crate
5. **Proof Generation**: Generates ZK proof using RISC Zero
6. **Result Storage**: Saves proof data to `proofs/` as JSON

### Output Files

**Email File** (`received_emails/20241124_153045_123456_sender_example_com.eml`):
- Raw email in RFC 5322 format
- Contains all headers, body, and attachments

**Proof File** (`proofs/20241124_153045_123456_sender_example_com.json`):
```json
{
  "timestamp": "2024-11-24T15:30:45Z",
  "from_domain": "example.com",
  "filename": "20241124_153045_123456_sender_example_com.eml",
  "output": {
    "from_domain_hash": "abc123...",
    "public_key_hash": "def456...",
    "verified": true,
    "hash_found": false
  }
}
```

## Security Considerations

### Essential Security Measures

1. **Rate Limiting**: Add connection limits to prevent spam floods
2. **Size Limits**: Configured via `max_message_size` (default: 50MB)
3. **Domain Filtering**: Use `allowed_domains` to restrict senders
4. **Non-root Execution**: Run as unprivileged user with `setcap`
5. **Log Monitoring**: Monitor logs for suspicious activity

### Recommended Additions

- **fail2ban**: Block IPs with repeated failed connections
- **SPF/DKIM Checks**: Already doing DKIM, consider adding SPF
- **TLS Support**: Add `rustls` for encrypted connections
- **Greylisting**: Temporarily reject unknown senders

### System Hardening

```bash
# Create dedicated user
sudo useradd -r -s /bin/false smtp-receiver

# Set directory permissions
sudo chown -R smtp-receiver:smtp-receiver received_emails/ proofs/
sudo chmod 750 received_emails/ proofs/

# Run as dedicated user
sudo -u smtp-receiver ./target/release/smtp-receiver config.toml
```

## Troubleshooting

### Port Already in Use

```bash
# Check what's using the port
sudo lsof -i :25

# Kill existing process if needed
sudo systemctl stop postfix  # If Postfix is running
```

### Permission Denied on Port 25

```bash
# Use setcap (see above) or run with sudo
sudo ./target/release/smtp-receiver config.toml
```

### DNS Resolution Issues

```bash
# Test DNS resolution
dig MX your-domain.com
dig mail.your-domain.com

# Test reverse DNS
dig -x <YOUR_VM_IP>
```

### DKIM Verification Failures

- Check that emails have DKIM-Signature headers
- Verify DNS can resolve DKIM public keys
- Check logs for specific error messages

## Development

### Testing Locally

```bash
# Terminal 1: Run server
RISC0_DEV_MODE=1 RUST_LOG=debug cargo run --bin smtp-receiver

# Terminal 2: Send test email
swaks --to test@localhost \
      --from test@example.com \
      --server localhost:2525 \
      --header "Subject: Test Email" \
      --body "This is a test"
```

### Using Real Emails

To test with real emails containing DKIM signatures:

1. Forward an email from Gmail/Outlook to your SMTP server
2. Check `received_emails/` for the saved `.eml` file
3. Check `proofs/` for the verification result
