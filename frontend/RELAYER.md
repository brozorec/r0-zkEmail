# OpenZeppelin Relayer Integration

This frontend uses the OpenZeppelin Relayer service for submitting Stellar/Soroban transactions.

## What is the Relayer?

The OpenZeppelin Relayer Channels Plugin enables parallel transaction submission on Stellar using channel accounts with fee bumping. It simplifies deploying Soroban smart contracts by automating transaction complexity.

**Service URL:** `https://channels.openzeppelin.com/testnet`

## Setup

### 1. Get an API Key

Contact OpenZeppelin or your project administrator to get a relayer API key.

Reference: https://github.com/OpenZeppelin/relayer-plugin-channels

### 2. Configure Environment Variables

Create a `.env` file in the `frontend/` directory:

```bash
cp .env.example .env
```

Edit `.env` and add your API key:

```env
VITE_RELAYER_API_KEY=your_actual_api_key_here
```

**Important:** Never commit the `.env` file to git. It's already in `.gitignore`.

### 3. Restart Dev Server

After adding the API key, restart the development server:

```bash
pnpm run dev
```

## How It Works

### Transaction Submission Flow

1. **Smart Account Kit** creates and signs transactions using WebAuthn passkeys
2. **Relayer Service** (`src/lib/relayer.ts`) encodes the transaction to XDR format
3. **OpenZeppelin Relayer** receives the request via API:
   - Validates the API key
   - Submits transaction using channel accounts
   - Handles fee bumping automatically
   - Returns transaction hash and status

### API Request Format

```typescript
POST https://channels.openzeppelin.com/testnet

Headers:
  Authorization: Bearer {VITE_RELAYER_API_KEY}
  Content-Type: application/json

Body:
{
  "params": {
    "func": "base64_encoded_host_function_xdr",
    "auth": ["base64_encoded_auth_entry_xdr", ...]
  }
}
```

### Response Format

```typescript
{
  "success": true,
  "data": {
    "hash": "transaction_hash",
    "status": "pending|success|failed",
    "transaction_id": "unique_identifier"
  }
}
```

## Usage in Code

### Submit a Transaction

```typescript
import { submitViaRelayer } from './lib/relayer';
import { xdr } from '@stellar/stellar-sdk';

// Build your host function and auth entries
const hostFunction = ...; // xdr.HostFunction
const authEntries = ...; // xdr.SorobanAuthorizationEntry[]

// Submit via relayer
const result = await submitViaRelayer(hostFunction, authEntries);

console.log('Transaction hash:', result.hash);
console.log('Status:', result.status);
```

### Check Relayer Health

```typescript
import { checkRelayerHealth } from './lib/relayer';

const isHealthy = await checkRelayerHealth();
if (!isHealthy) {
  console.error('Relayer service is down');
}
```

## Error Handling

The relayer service will throw errors in these cases:

- **No API key configured:** `VITE_RELAYER_API_KEY` not set
- **Invalid API key:** 401 Unauthorized
- **Rate limit exceeded:** 429 Too Many Requests
- **Transaction failed:** Error details in response

Always wrap relayer calls in try-catch blocks:

```typescript
try {
  const result = await submitViaRelayer(hostFunction, authEntries);
  // Handle success
} catch (error) {
  console.error('Transaction submission failed:', error);
  // Show user-friendly error message
}
```

## Fee Management

The relayer service has built-in fee tracking and limits:

- **Default fee limit:** Configured per API key
- **Reset period:** Fees reset periodically
- **Fee bumping:** Automatic for stuck transactions

Monitor your fee usage through the relayer's management API (admin access required).

## Architecture

```
┌─────────────┐
│   Browser   │
│  (Frontend) │
└──────┬──────┘
       │ 1. Sign with passkey
       │
┌──────▼──────────────┐
│  Smart Account Kit  │
│   (WebAuthn SDK)    │
└──────┬──────────────┘
       │ 2. Create signed transaction
       │
┌──────▼──────────────┐
│ Relayer Service API │
│  (OpenZeppelin)     │
└──────┬──────────────┘
       │ 3. Submit to Stellar
       │
┌──────▼──────────────┐
│  Stellar Network    │
│     (Testnet)       │
└─────────────────────┘
```

## Troubleshooting

### "Relayer API key not configured"

**Solution:** Set `VITE_RELAYER_API_KEY` in your `.env` file.

### "401 Unauthorized"

**Solution:** Your API key is invalid. Check with your administrator.

### "Transaction submission failed"

**Solution:** Check the error details. Common causes:
- Insufficient balance for fees
- Invalid transaction structure
- Contract execution failure

### "Relayer service is down"

**Solution:** Check https://status.openzeppelin.com or contact support.

## Development vs Production

### Development (Testnet)

- Endpoint: `https://channels.openzeppelin.com/testnet`
- Network: Stellar Testnet
- Free testnet XLM available from Friendbot

### Production (Mainnet)

When ready for mainnet:

1. Update relayer endpoint in `src/lib/relayer.ts`
2. Get a mainnet API key
3. Update network configuration in `src/config.ts`
4. Test thoroughly on testnet first!

## References

- [Relayer Plugin Channels](https://github.com/OpenZeppelin/relayer-plugin-channels)
- [Example Implementation (Rust)](https://github.com/brozorec/smart-account-sign/blob/main/stellar-smart-account/src/relayer.rs)
- [Smart Account Kit](./vendor/stellar-contracts/smart-account-kit)
- [Stellar Documentation](https://developers.stellar.org/)
