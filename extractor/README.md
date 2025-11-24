# Groth16 Proof Extractor

This tool extracts proof data from a RISC Zero Groth16 receipt for submission to Stellar verifier contracts.

## Usage

```bash
# Extract from default location (groth16_receipt.bin)
cargo run --bin extractor

# Extract from custom path
cargo run --bin extractor /path/to/groth16_receipt.bin
```

## Output Files

The extractor generates the following files:

- **`groth16_proof.json`** - Human-readable JSON containing:
  - `seal`: Groth16 proof as hex string (260 bytes)
  - `journal_hex`: Raw public outputs as hex string
  - `journal_digest`: SHA-256 digest of the journal (required by verifier)
  - `image_id`: Program identifier (digest of the zkVM program)
  - `claim_digest`: Claim digest

## Stellar Contract Integration

For the [Nethermind Stellar RISC0 Verifier](https://github.com/NethermindEth/stellar-risc0-verifier), you'll need:

1. **`image_id`** - Identifies which zkVM program was executed (32 bytes)
2. **`journal_digest`** - SHA-256 digest of the journal bytes (32 bytes, NOT the raw journal)
3. **`seal`** - The Groth16 proof as hex string (260 bytes: 4-byte selector + 256-byte proof)

All three values are available in the generated `groth16_proof.json` file.

**Important**: The Stellar verifier expects the SHA-256 digest of the journal, not the raw journal bytes. The extractor automatically computes this digest for you.

## Example

```bash
$ cargo run --bin extractor groth16_receipt.bin
```
