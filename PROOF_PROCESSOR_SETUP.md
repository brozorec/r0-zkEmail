# Proof Processor Setup Guide

## Overview

The `proof-processor` is a standalone service that monitors the proof directory for new RISC Zero receipts, extracts proof data, and submits transactions to the Stellar blockchain via OpenZeppelin's managed relayer service.

## Architecture

```
┌──────────────────┐
│  SMTP Receiver   │
│  - Receives      │
│    email pairs   │
│  - Generates     │
│    ZK proofs     │
│  - Saves         │
│    receipts      │
└────────┬─────────┘
         │
         ▼
    ./proofs/
    receipt_*.bin
         │
         ▼
┌──────────────────┐
│ Proof Processor  │
│ (periodic scan)  │
│  - Extracts      │
│    proof data    │
│  - Submits to    │
│    blockchain    │
└────────┬─────────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
./proofs/  ./proofs/
processed/  failed/
```

## Setup Instructions

### 1. Create Configuration File

Copy the example configuration:

```bash
cd proof-processor
cp config.toml.example config.toml
```

Edit `config.toml` with your settings:

```toml
[processor]
proof_dir = "./proofs"                    # Match SMTP receiver's proof_dir
processed_dir = "./proofs/processed"
failed_dir = "./proofs/failed"
scan_interval_secs = 60                   # Check every minute
retry_attempts = 3
retry_delay_secs = 5

[stellar]
network = "testnet"                       # testnet | futurenet | mainnet
contract_address = "CXXX..."              # Your EmailPaymentGateway contract

[relayer]
api_url = "https://channels.openzeppelin.com/testnet"
api_key = "your-openzeppelin-api-key"
```

### 2. Get OpenZeppelin API Key

1. Visit [OpenZeppelin Defender](https://defender.openzeppelin.com/)
2. Create an account or sign in
3. Navigate to Relayer settings
4. Generate an API key for Stellar testnet
5. Copy the API key to your `config.toml`

### 3. Deploy EmailPaymentGateway Contract

Before running the processor, you need to deploy the `EmailPaymentGateway` contract to Stellar:

```bash
cd contract
# Deploy contract and note the contract address
# Update config.toml with the contract address
```

### 4. Build the Processor

```bash
cargo build --release --bin proof-processor
```

### 5. Run the Processor

```bash
# With default config.toml in current directory
cargo run --release --bin proof-processor

# With custom config path
cargo run --release --bin proof-processor /path/to/config.toml

# With debug logging
RUST_LOG=debug cargo run --release --bin proof-processor
```

## How It Works

### Startup Behavior

1. Loads configuration from `config.toml`
2. Validates configuration (contract address, API key, network)
3. Creates necessary directories (proof_dir, processed_dir, failed_dir)
4. Processes any existing `.bin` files in proof_dir
5. Starts periodic scanner

### Processing Flow

For each receipt file:

1. **Extract Proof Data**
   - Deserialize RISC Zero receipt
   - Extract Groth16 seal (260 bytes)
   - Extract image_id (32 bytes)
   - Extract journal bytes
   - Decode PaymentReceipt from journal
   - Compute journal SHA-256 digest

2. **Validate**
   - Check that `payment_receipt.verified == true`
   - Validate seal and image_id sizes

3. **Submit to Blockchain**
   - Build `verify_and_pay(seal, image_id, journal)` invocation
   - Encode as XDR
   - Submit to OpenZeppelin relayer
   - Relayer handles transaction signing and submission

4. **Handle Result**
   - **Success**: Move receipt to `processed_dir`
   - **Failure**: Retry up to `retry_attempts` times
   - **Persistent Failure**: Move receipt to `failed_dir`

### Retry Logic

- Both extraction and submission have retry logic
- Configurable retry attempts (default: 3)
- Configurable delay between retries (default: 5 seconds)
- Failed receipts moved to `failed_dir` for manual inspection

## Directory Structure

```
./proofs/
├── receipt_20241129_120000_abc123.bin    # New receipts (pending)
├── processed/
│   └── receipt_20241129_110000_xyz789.bin  # Successfully submitted
└── failed/
    └── receipt_20241129_100000_def456.bin  # Failed after retries
```

## Monitoring

### Log Levels

- **INFO**: Processing status, successes, failures
- **DEBUG**: Detailed extraction data, transaction details
- **ERROR**: Failures with full error context

### Example Logs

```
INFO  Processing receipt: ./proofs/receipt_20241129_120000.bin
INFO  Payment Receipt - Sender: GXXX..., Amount: 1000000000, Nonce: 12345, Verified: true
INFO  Submitting proof to blockchain
INFO  Transaction submitted successfully - Hash: abc123..., Status: pending
INFO  Successfully processed receipt: ./proofs/receipt_20241129_120000.bin
```

### Monitoring Failures

Check `./proofs/failed/` directory for receipts that failed after all retries:

```bash
ls -la ./proofs/failed/
```

Inspect logs for error details:

```bash
RUST_LOG=debug cargo run --bin proof-processor 2>&1 | tee processor.log
```

## Integration with SMTP Receiver

The SMTP receiver and proof processor work together:

1. **SMTP Receiver** (`smtp-receiver/config.toml`):
   ```toml
   [storage]
   proof_dir = "./proofs"  # Must match processor's proof_dir
   ```

2. **Proof Processor** (`proof-processor/config.toml`):
   ```toml
   [processor]
   proof_dir = "./proofs"  # Same directory
   ```

Both services can run independently:
- SMTP receiver generates receipts
- Proof processor picks them up and submits them

## Extensibility

### Custom Notifications

The processor includes a `NotificationHandler` trait for future alert mechanisms:

```rust
use proof_processor::{NotificationHandler, ReceiptProcessor};
use std::path::Path;

struct EmailNotifier;

impl NotificationHandler for EmailNotifier {
    fn on_success(&self, receipt_path: &Path, tx_data: &RelayerResponse) {
        // Send success email/webhook
    }
    
    fn on_failure(&self, receipt_path: &Path, error: &anyhow::Error) {
        // Send failure alert
    }
}

// Use custom handler
let processor = ReceiptProcessor::new(config)
    .with_notification_handler(Box::new(EmailNotifier));
```

## Production Deployment

### Systemd Service

Create `/etc/systemd/system/proof-processor.service`:

```ini
[Unit]
Description=Proof Processor for zkEmail
After=network.target

[Service]
Type=simple
User=zkemail
WorkingDirectory=/opt/zkemail/proof-processor
ExecStart=/opt/zkemail/proof-processor/target/release/proof-processor /opt/zkemail/proof-processor/config.toml
Restart=always
RestartSec=10
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable proof-processor
sudo systemctl start proof-processor
sudo systemctl status proof-processor
```

### Docker Deployment

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin proof-processor

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/proof-processor /usr/local/bin/
COPY config.toml /etc/proof-processor/config.toml
CMD ["proof-processor", "/etc/proof-processor/config.toml"]
```

## Troubleshooting

### Receipt Not Processing

1. Check file permissions on proof_dir
2. Verify receipt file has `.bin` extension
3. Check logs for extraction errors
4. Ensure receipt is in Groth16 format (not dev/fake mode)

### Submission Failures

1. Verify OpenZeppelin API key is valid
2. Check contract address is correct
3. Ensure network matches (testnet/futurenet/mainnet)
4. Check relayer service status
5. Verify contract is deployed and accessible

### High Failure Rate

1. Check `./proofs/failed/` for common error patterns
2. Increase `retry_attempts` and `retry_delay_secs`
3. Verify network connectivity to relayer
4. Check Stellar network status

## Next Steps

1. Set up monitoring/alerting (implement NotificationHandler)
2. Add metrics collection (Prometheus/Grafana)
3. Implement database for tracking submission history
4. Add webhook support for external integrations
5. Create admin dashboard for monitoring
