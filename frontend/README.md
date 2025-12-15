# Email Pay Frontend

A modern web application for sending USDC via email, built with React, TypeScript, and Stellar blockchain.

## Overview

Email Pay allows users to send USDC cryptocurrency using just an email address. Recipients don't need an existing wallet - one is automatically created for them using secure WebAuthn passkeys (FaceID, TouchID, or PIN).

## Tech Stack

- **Frontend**: React 18 + TypeScript + Vite
- **Styling**: Tailwind CSS v3.4
- **Blockchain**: Stellar/Soroban (Testnet)
- **Authentication**: WebAuthn (Passkeys)
- **Smart Accounts**: OpenZeppelin Smart Account Kit
- **Transaction Relay**: OpenZeppelin Relayer Service

## Prerequisites

- Node.js 18+ or Bun
- pnpm (recommended) or npm
- OpenZeppelin Relayer API key (for transaction submission)

## Installation

1. Clone the repository and navigate to the frontend directory:
```bash
cd frontend
```

2. Install dependencies:
```bash
pnpm install
# or
npm install
```

3. Configure environment variables:
```bash
cp .env.example .env
```

Edit `.env` and add your relayer API key:
```env
VITE_RELAYER_API_KEY=your_relayer_api_key_here
```

## Development

Start the development server:
```bash
pnpm run dev
```

The app will be available at `http://localhost:5173/`

## Building

Build for production:
```bash
pnpm run build
```

Preview production build:
```bash
pnpm run preview
```

## Project Structure

```
frontend/
├── src/
│   ├── components/        # Reusable UI components
│   │   ├── BalanceDisplay.tsx
│   │   ├── CreatePasskeyButton.tsx
│   │   ├── CreateWalletForm.tsx
│   │   ├── EmailPreviewPopup.tsx
│   │   ├── Header.tsx
│   │   ├── SendForm.tsx
│   │   └── Toast.tsx
│   ├── context/          # React context providers
│   │   └── WalletContext.tsx
│   ├── hooks/            # Custom React hooks
│   │   ├── useBalance.ts
│   │   ├── useBalanceCache.ts
│   │   └── useToast.ts
│   ├── lib/              # Core libraries
│   │   ├── email.ts               # Email template generation
│   │   ├── format.ts              # Formatting utilities
│   │   ├── relayer.ts             # Relayer API integration
│   │   ├── smartAccount.ts        # Smart Account Kit setup
│   │   ├── smartAccountRelayer.ts # SA + Relayer integration
│   │   └── stellar.ts             # Stellar SDK integration
│   ├── pages/            # Page components
│   │   ├── Landing.tsx
│   │   ├── Receive.tsx
│   │   └── Send.tsx
│   ├── types/            # TypeScript types
│   │   └── index.ts
│   ├── App.tsx           # Main app component
│   ├── config.ts         # App configuration
│   └── main.tsx          # Entry point
├── public/               # Static assets
├── .env.example          # Environment variables template
├── RELAYER.md           # Relayer integration guide
└── README.md            # This file
```

## Features

### Sender Flow (Send Page)
1. Create a passkey-based wallet (or connect existing)
2. View USDC balance
3. Mint testnet USDC for testing
4. Send USDC to any email address
5. Generate payment email for recipient

### Receiver Flow (Receive Page)
1. Click payment link from email
2. Create secure passkey (FaceID/TouchID/PIN)
3. Send confirmation email
4. Receive wallet address and access funds

### Landing Page
- Marketing hero section
- How it works explanation
- Feature highlights
- Call-to-action buttons

## Configuration

Edit `src/config.ts` to customize:

```typescript
export const config = {
  network: "testnet",
  rpcUrl: "https://soroban-testnet.stellar.org",
  networkPassphrase: "Test SDF Network ; September 2015",

  // Contract addresses
  accountWasmHash: "...",
  webauthnVerifierAddress: "...",
  usdcContractAddress: "...",

  // App settings
  checkEmail: "check@mailspay.cc",
  appName: "Email Pay"
};
```

## Key Integrations

### Smart Account Kit
Located in `vendor/stellar-contracts/smart-account-kit` (git submodule)

Provides:
- WebAuthn passkey wallet creation
- Transaction signing with biometrics
- IndexedDB credential storage

### OpenZeppelin Relayer
See `RELAYER.md` for detailed integration guide

Handles:
- Gasless transactions (users don't pay fees)
- Channel account management
- Automatic fee bumping
- Transaction submission

### Stellar/Soroban
- Balance queries via RPC simulation
- USDC token contract interaction
- Smart contract deployment

## Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `VITE_RELAYER_API_KEY` | OpenZeppelin relayer API key | Yes |

## Known Limitations

1. **Authorization Entry Extraction**: The Smart Account Kit signing to relayer auth entry extraction is not yet implemented. See `src/lib/smartAccountRelayer.ts` for TODO.

2. **Testnet Only**: Currently configured for Stellar testnet only.

3. **Email Backend**: Email sending is simulated (shows email preview). Requires backend integration for production.

## Troubleshooting

### Build Errors

**Tailwind PostCSS errors**: Make sure you're using Tailwind CSS v3.4, not v4:
```bash
pnpm remove tailwindcss
pnpm add -D tailwindcss@^3.4.0
```

**Stellar SDK import errors**: Use lowercase `rpc` export:
```typescript
import { rpc } from '@stellar/stellar-sdk';
const server = new rpc.Server(config.rpcUrl);
```

### Runtime Errors

**Passkey creation fails**:
- Check that you're on HTTPS or localhost
- Ensure WebAuthn is supported in your browser
- Try a different authenticator (FaceID/TouchID/PIN)

**Balance fetching fails**:
- Verify RPC URL is accessible
- Check contract addresses in config
- Ensure account exists on testnet

**Relayer errors**:
- Verify API key is set in `.env`
- Check API key permissions
- Review `RELAYER.md` for troubleshooting

## Contributing

1. Ensure code passes TypeScript checks: `pnpm run build`
2. Follow existing code style and component patterns
3. Add error handling for all async operations
4. Update documentation for new features

## License

See the main project LICENSE file.
