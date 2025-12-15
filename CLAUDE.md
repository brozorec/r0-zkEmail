# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

r0-zkEmail is a RISC Zero project that enables zero-knowledge verification of DKIM email signatures. It allows proving email authenticity without revealing email content through cryptographic proofs.

## Common Development Commands

### Building
```bash
# Build all workspace members
cargo build --release

# Build specific components
cargo build --release --bin host
cargo build --release --bin smtp-receiver
cargo build --release --bin proof-processor
cargo build --release --package prove-api
```

### Running Components

**Core Host (Email Verification):**
```bash
# Run in development mode with fake proofs (faster)
RISC0_DEV_MODE=1 RUST_LOG=info cargo run --release -- <FROM_DOMAIN> <EMAIL_PATH>

# Example with real email
RISC0_DEV_MODE=1 RUST_LOG=info cargo run --release -- gmail.com example.eml
```

**SMTP Receiver:**
```bash
# Generate default configuration
cargo run --bin smtp-receiver -- --generate-config config.toml

# Run in development mode
RISC0_DEV_MODE=1 RUST_LOG=info cargo run --bin smtp-receiver -- config.toml

# Run with debug logging
RUST_LOG=debug cargo run --bin smtp-receiver
```

**Prove API (REST server):**
```bash
# Run locally
cargo run --package prove-api

# Run with CUDA acceleration
cargo run --package prove-api --features cuda

# Custom port
PORT=3000 cargo run --package prove-api
```

**Proof Processor:**
```bash
# Run with config file
cargo run --bin proof-processor /path/to/config.toml

# Debug mode
RUST_LOG=debug cargo run --bin proof-processor
```

**Extractor (Receipt Processing):**
```bash
# Extract from receipt file
cargo run --bin extractor /path/to/groth16_receipt.bin
```

### Testing
```bash
# Run all tests
cargo test

# Test specific component
cargo test --package smtp-receiver
```

### Environment Variables

- `RISC0_DEV_MODE=1`: Use fake proofs for faster development
- `RISC0_DEV_MODE=0`: Generate real ZK proofs (slower, requires more resources)
- `RUST_LOG=debug|info|warn|error`: Set logging level

## Architecture Overview

This is a Rust workspace with multiple interconnected components:

### Core Components

- **`host/`**: Main RISC Zero host application that orchestrates DKIM verification and ZK proof generation
- **`methods/`**: RISC Zero guest programs that run inside the zkVM for proof generation
- **`core/`**: Shared core logic and types used across components
- **`email-parser/`**: Email parsing utilities and DKIM signature extraction

### Applications

- **`smtp-receiver/`**: Standalone SMTP server that receives emails and automatically triggers verification
- **`prove-api/`**: REST API server for generating ZK proofs via HTTP endpoints (designed for RunPod deployment)
- **`proof-processor/`**: Batch processor for handling multiple email verifications
- **`extractor/`**: Utility for extracting and processing receipt files

### Supporting Components

- **`contract/`**: Stellar/Soroban smart contract integration (for on-chain verification)

### Key Data Flow

1. **Email Input**: Raw email files (.eml format) containing DKIM signatures
2. **Domain Extraction**: Extract sender domain (e.g., gmail.com, outlook.com)
3. **DKIM Verification**: Parse and verify DKIM signatures against DNS records
4. **ZK Proof Generation**: Use RISC Zero zkVM to generate cryptographic proofs
5. **Output**: Receipt files and proof data in various formats

### Configuration

- **`config.toml`**: Main configuration file for SMTP receiver and proof processor
- **`rust-toolchain.toml`**: Specifies stable Rust toolchain with rustfmt and rust-src components
- **Development**: Use `RISC0_DEV_MODE=1` for faster development with mock proofs
- **Production**: Use `RISC0_DEV_MODE=0` for real ZK proof generation

### Features and Build Variations

- **CUDA Support**: Available via `--features cuda` for GPU-accelerated proof generation
- **Verify Feature**: Some components have optional `verify` feature for local vs. remote verification
- **Docker**: Containerized deployments available, especially for prove-api component

The project follows a modular design where each component can be run independently or as part of an integrated email verification pipeline.
