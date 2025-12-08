# Proof Processor

Automated service that processes RISC Zero receipts and submits them to the Stellar blockchain via OpenZeppelin's relayer service.

## Features

- **Automatic Receipt Processing**: Periodically scans for new receipt files
- **Proof Extraction**: Extracts Groth16 proofs and journal data from receipts
- **On-chain Submission**: Submits proofs to Stellar smart contracts via relayer
- **Retry Logic**: Configurable retry attempts with exponential backoff
- **File Management**: Automatically organizes receipts into processed/failed directories
- **Extensible Notifications**: Hook points for future alert mechanisms

## Architecture

```
proof-processor/
├── config.rs       # Configuration management
├── extractor.rs    # Proof data extraction from receipts
├── relayer.rs      # OpenZeppelin relayer client
├── processor.rs    # Orchestration and retry logic
└── main.rs         # Directory watcher and entry point
```

## Configuration

Copy `config.toml.example` to `config.toml` and configure:

```toml
[processor]
proof_dir = "./proofs"
processed_dir = "./proofs/processed"
failed_dir = "./proofs/failed"
scan_interval_secs = 60
retry_attempts = 3
retry_delay_secs = 5

[stellar]
network = "testnet"
contract_address = "CXXX..."

[relayer]
api_url = "https://channels.openzeppelin.com/testnet"
api_key = "your-api-key"
```

### Configuration Options

#### `[processor]`
- **`proof_dir`**: Directory to watch for new `.bin` receipt files
- **`processed_dir`**: Where successfully processed receipts are moved
- **`failed_dir`**: Where failed receipts are moved
- **`scan_interval_secs`**: How often to scan for new receipts (default: 60)
- **`retry_attempts`**: Number of retry attempts for failures (default: 3)
- **`retry_delay_secs`**: Delay between retries (default: 5)

#### `[stellar]`
- **`network`**: Stellar network (`testnet`, `futurenet`, or `mainnet`)
- **`contract_address`**: EmailPaymentGateway contract address

#### `[relayer]`
- **`api_url`**: OpenZeppelin relayer endpoint
- **`api_key`**: Your relayer API key

## Usage

### Run with default config

```bash
cargo run --bin proof-processor
```

### Run with custom config

```bash
cargo run --bin proof-processor /path/to/config.toml
```

### With logging

```bash
RUST_LOG=debug cargo run --bin proof-processor
```

## How It Works

1. **Startup**: Processes any existing `.bin` files in `proof_dir`
2. **Periodic Scan**: Every `scan_interval_secs`, scans for new receipts
3. **For each receipt**:
   - Extract proof data (seal, image_id, journal)
   - Validate payment receipt (must be verified)
   - Build `verify_and_pay` contract invocation
   - Submit to Stellar via relayer (with retries)
   - Move to `processed_dir` on success or `failed_dir` on failure

## Receipt Processing Flow

```
┌─────────────────┐
│  New receipt    │
│  in proof_dir   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Extract proof  │
│  data from      │
│  receipt        │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Validate       │
│  payment data   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Build contract │
│  invocation     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Submit to      │
│  relayer        │
│  (with retries) │
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
┌────────┐ ┌────────┐
│Success │ │ Failed │
│→ Move  │ │→ Move  │
│to      │ │to      │
│process/│ │failed/ │
└────────┘ └────────┘
```

## Extending with Notifications

The processor includes a `NotificationHandler` trait for future alert mechanisms:

```rust
pub trait NotificationHandler: Send + Sync {
    fn on_success(&self, receipt_path: &Path, tx_data: &RelayerResponse);
    fn on_failure(&self, receipt_path: &Path, error: &anyhow::Error);
}
```

To add custom notifications:

1. Implement the `NotificationHandler` trait
2. Pass it to the processor:

```rust
let processor = ReceiptProcessor::new(config)
    .with_notification_handler(Box::new(MyCustomHandler));
```

## Integration with SMTP Receiver

The SMTP receiver saves receipts to the configured `proof_dir`. The proof-processor automatically picks them up and processes them.

Typical workflow:
1. SMTP receiver gets email pair
2. Generates ZK proof
3. Saves receipt to `./proofs/receipt_*.bin`
4. Proof-processor detects new file
5. Extracts and submits to blockchain

## Error Handling

- **Extraction failures**: Retried up to `retry_attempts` times
- **Submission failures**: Retried up to `retry_attempts` times
- **Persistent failures**: Receipt moved to `failed_dir` for manual inspection
- **All errors logged** with context for debugging

## Monitoring

Check the logs for:
- `INFO`: Processing status, successes, failures
- `DEBUG`: Detailed extraction and submission data
- `ERROR`: Failures with full error context

## Production Deployment

1. Set `RUST_LOG=info` for production logging
2. Configure proper `proof_dir` path (shared with SMTP receiver)
3. Set secure `api_key` in config
4. Run as a systemd service or in a container
5. Monitor logs for failures
6. Implement custom `NotificationHandler` for alerts

## Dependencies

- **risc0-zkvm**: Receipt deserialization and proof extraction
- **stellar-xdr**: Transaction building for Stellar
- **reqwest**: HTTP client for relayer API
- **tokio**: Async runtime
- **notify**: File system watching (optional, currently using periodic scan)
