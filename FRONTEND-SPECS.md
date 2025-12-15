# Email Pay Frontend — Specification

## Overview

A mobile-first web application enabling USDC transfers via email on Stellar testnet. Recipients don't need existing wallets—they create one with a single tap using passkeys.

**Tagline:** "Your inbox is now a wallet"

## Tech Stack

- **Build**: Vite
- **Framework**: React
- **Styling**: Tailwind CSS
- **Stellar SDK**: `@stellar/stellar-sdk`
- **Smart Account**: `@openzeppelin/smart-account-kit` (from https://github.com/kalepail/stellar-contracts/tree/kalepail-edit/smart-account-kit, vendored as a git submodule under `/vendor` because it is not yet on npm)

## Configuration

```ts
// src/config.ts
export const config = {
  network: "testnet",
  rpcUrl: "https://soroban-testnet.stellar.org",
  networkPassphrase: "Test SDF Network ; September 2015",
  nativeTokenContract: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",

  // Smart Account Kit config
  accountWasmHash: "1fcf7b001c78efeebff016a2a835c2a617b4c1b61dfdb86d5e3bbad31c4c9988",
  webauthnVerifierAddress: "CBEO6Q7UXBIQIHQR42RXETMYDKW7GABRX2O4UVW6O6YQOHROYWZJCOXZ",

  // Contract addresses
  usdcContractAddress: "CBKASTI5ATUW5BJOOCDYINSVOCXXGWAZNXE3MGQCCYZG5MKJOPIR5JVU",
  paymentGatewayAddress: "<CONFIGURE_GATEWAY_CONTRACT>",

  // App config
  checkEmail: "check@mailspay.cc",
  appName: "Email Pay"
} as const;
```

---

## Smart Account Kit Integration

### SDK Initialization

```ts
// src/lib/smartAccount.ts
import { SmartAccountKit, IndexedDBStorage } from 'smart-account-kit';
import { config } from '../config';

export const kit = new SmartAccountKit({
  rpcUrl: config.rpcUrl,
  networkPassphrase: config.networkPassphrase,
  accountWasmHash: config.accountWasmHash,
  webauthnVerifierAddress: config.webauthnVerifierAddress,
  storage: new IndexedDBStorage(),
});

export type { CreateWalletResult } from 'smart-account-kit';
```

### Creating a New Wallet

```ts
// WalletContext.tsx
const result = await kit.createWallet(
  'Email Pay',
  email,
  { autoSubmit: true }
);

setContractId(result.contractId);
setCredentialId(result.credentialId);
setSenderEmail(email);
localStorage.setItem('emailpay:senderEmail', email);
```

### Connecting to Existing Wallet

```ts
// WalletContext.tsx
const connectWallet = async (contractIdParam?: string) => {
  setIsConnecting(true);
  try {
    const result = await kit.connectWallet(
      contractIdParam ? { contractId: contractIdParam } : undefined
    );
    if (result) {
      setContractId(result.contractId);
      setCredentialId(result.credentialId);
    }
  } finally {
    setIsConnecting(false);
  }
};
```

### Signing Transactions

```ts
// For token transfers, use the built-in transfer method
const result = await kit.transfer(
  config.usdcContractAddress,  // Token contract
  recipientAddress,            // Recipient C or G address
  amount                       // Amount as string
);
```

### Getting Balance

```ts
// src/lib/stellar.ts
export async function fetchBalance(address: string): Promise<bigint> {
  const contract = new Contract(config.usdcContractAddress);
  const simulationKeypair = getSimulationKeypair();
  const sourceAccount = await server.getAccount(simulationKeypair.publicKey());

  const transaction = new TransactionBuilder(sourceAccount, {
    fee: BASE_FEE,
    networkPassphrase: config.networkPassphrase,
  })
    .addOperation(contract.call('balance', nativeToScVal(address, { type: 'address' })))
    .setTimeout(30)
    .build();

  const response = await server.simulateTransaction(transaction);
  if (!rpc.Api.isSimulationSuccess(response) || !response.result?.retval) {
    return 0n;
  }

  return scValToBigInt(response.result.retval);
}
```

### Minting Testnet USDC

```ts
// src/pages/Send.tsx
const handleMintUsdc = async () => {
  if (!contractId) return;
  setIsMinting(true);
  try {
    const { mintUsdcViaRelayer } = await import('../lib/smartAccountRelayer');
    const mintAmount = 100_0000000n;
    await mintUsdcViaRelayer(contractId, mintAmount);
    success('Minted 100 USDC via OpenZeppelin relayer');
    setTimeout(() => refetchBalance(), 2000);
  } finally {
    setIsMinting(false);
  }
};
```

### Events

```ts
// Listen for wallet connection
kit.events.on('walletConnected', ({ contractId }) => {
  console.log('Connected:', contractId);
});

// Listen for transaction submission
kit.events.on('transactionSubmitted', ({ hash, success }) => {
  // Show notification
});
```

### Error Handling

```ts
import { 
  WalletNotConnectedError,
  SimulationError,
  SubmissionError,
  WebAuthnError 
} from '@openzeppelin/smart-account-kit';

try {
  await kit.createWallet(...);
} catch (error) {
  if (error instanceof WebAuthnError) {
    // User cancelled passkey creation or not supported
  } else if (error instanceof SimulationError) {
    // Transaction simulation failed
  } else if (error instanceof SubmissionError) {
    // Transaction submission failed
  }
}
```

---

## User Stories

### US-1: Learn About the Product

**As a** visitor  
**I want to** understand what Email Pay does and how it works  
**So that** I can decide whether to use it

**Acceptance Criteria:**
- [ ] Landing page accessible via `/` (root) and header navigation (as "Home")
- [ ] Hero section displays tagline "Your inbox is now a wallet"
- [ ] Hero includes subtext explaining the value proposition
- [ ] Hero includes CTA button linking to Send page (`/send`)
- [ ] Animated visual flow diagram shows the payment journey: `You → ✉️ Email → ✨ Magic → 💰 They get paid`
- [ ] Tabbed explanation section with two tabs:
  - "For humans": 3 simple steps with icons, plain English, no jargon
  - "For nerds": technical explanation covering DKIM verification, ZK proofs, Stellar smart accounts, passkeys
- [ ] Page is fully responsive (mobile-first)

---

### US-2: Create a Smart Account

**As a** new user  
**I want to** create a smart account using a passkey  
**So that** I can send and receive USDC

**Acceptance Criteria:**
- [ ] Send page (`/send`) displays CTA "Create Wallet" when no account exists
- [ ] Before passkey creation, prompt user for their email address (required)
- [ ] Validate email format before proceeding
- [ ] Clicking CTA calls `kit.createWallet(appName, userEmail, { autoSubmit: true })`
- [ ] WebAuthn passkey creation prompt appears (FaceID/TouchID/PIN)
- [ ] On success, store returned `contractId`, `credentialId`, and user's email in app state
- [ ] Store email in localStorage for future sessions
- [ ] Toast notification: "Account created"
- [ ] User sees dashboard view with their new account

**Error States:**
- [ ] Invalid email format → show validation error, don't proceed
- [ ] User cancels passkey prompt → show "Wallet creation cancelled" message
- [ ] WebAuthn not supported → show "Passkeys not supported on this device"
- [ ] Transaction fails → show error with retry option

---

### US-3: Access Existing Account

**As a** returning user  
**I want to** access my previously created account  
**So that** I can view my balance and send payments

**Acceptance Criteria:**
- [ ] On `/send` page load, check URL for `?account=<address>` query param
- [ ] If URL param present, call `kit.connectWallet({ contractId: address })`
- [ ] If no URL param, call `kit.connectWallet()` for silent session restore
- [ ] If silent restore fails and accounts exist in storage, show account selector
- [ ] If one account → display dashboard for it
- [ ] If multiple accounts → show dropdown selector, default to most recently used
- [ ] Toast notification: "Welcome back" (on successful restore)
- [ ] Display account address (truncated) in header

**Error States:**
- [ ] Session expired → prompt to reconnect with passkey
- [ ] Contract not found → show error, offer to create new account

---

### US-4: Manage Stored Accounts Dropdown

**As a** returning user  
**I want to** access all my stored accounts from the header dropdown  
**So that** I can switch wallets to view balances and send payments

**Acceptance Criteria:**
- [ ] When at least one stored account exists, the account indicator in the header becomes a clickable control
- [ ] Clicking the account indicator opens a dropdown listing all stored contract addresses (truncated) pulled from Smart Account Kit storage/localStorage
- [ ] Dropdown entries display the address plus a "Last used" timestamp when available
- [ ] Selecting a stored account calls `kit.connectWallet({ contractId })`, updates WalletContext state, and closes the dropdown
- [ ] The currently active account is visually distinguished in the dropdown
- [ ] If only one stored account exists, dropdown still opens but shows "Only one account saved" helper text
- [ ] Dropdown supports keyboard navigation and closes on outside click or Escape

**Error States:**
- [ ] Stored accounts fail to load → show toast "Unable to load saved accounts" and keep dropdown disabled
- [ ] Selecting an account fails → show toast "Failed to switch account" and keep previous account active

---

### US-5: Create New Account from Dropdown

**As a** returning user  
**I want to** create a new account directly from the stored-accounts dropdown  
**So that** I can add additional wallets without leaving the header interaction

**Acceptance Criteria:**
- [ ] Dropdown includes a footer CTA "Create new account"
- [ ] Clicking the CTA prompts for an email (prefilled with stored sender email when available) and runs the Smart Account Kit `createWallet` flow
- [ ] New account creation uses the same validation and toasts defined in US-2
- [ ] On success, the newly created account is added to storage, selected as active, and the dropdown closes with a toast "Account created"
- [ ] Dropdown automatically refreshes to show the new contract address

**Error States:**
- [ ] User cancels creation → dropdown remains open with info toast "Wallet creation cancelled"
- [ ] Creation fails → show toast with error message and keep existing account selected

---

### US-6: View USDC Balance

**As a** user with a smart account  
**I want to** see my current USDC balance  
**So that** I know how much I can send

**Acceptance Criteria:**
- [ ] Dashboard displays current USDC balance for connected account
- [ ] Balance fetched by calling `balance(address)` on USDC contract via Soroban RPC
- [ ] Balance is polled every 5 seconds
- [ ] Previous balance cached in localStorage with timestamp
- [ ] If new balance > cached → toast: "Account credited with X USDC"
- [ ] If new balance < cached → toast: "Sent X USDC"
- [ ] If balance == 0 → display "Mint testnet USDC" button
- [ ] Display balance formatted with 7 decimal places, trim trailing zeros

**Implementation Notes:**
```ts
// Balance is returned in stroops (1 USDC = 10^7 stroops)
const displayBalance = (stroops: bigint) => {
  const usdc = Number(stroops) / 10_000_000;
  return usdc.toFixed(7).replace(/\.?0+$/, '');
};
```

---

### US-7: Mint Testnet USDC

**As a** user with zero balance  
**I want to** mint testnet USDC  
**So that** I can test sending payments

**Acceptance Criteria:**
- [ ] "Mint testnet USDC" button visible only when balance == 0
- [ ] Clicking button calls `mint(to, amount)` on the testnet USDC contract
- [ ] Default mint amount: 100 USDC (100_0000000 stroops)
- [ ] Loading state shown during transaction
- [ ] On success, balance updates via polling
- [ ] Toast: "Minted 100 USDC"

**Implementation Notes:**
```ts
// The testnet USDC contract allows anyone to call mint
const mintUsdc = async (toAddress: string, amount: bigint) => {
  const contract = new Contract(config.usdcContractAddress);
  const tx = new TransactionBuilder(sourceAccount, { fee: BASE_FEE })
    .addOperation(contract.call('mint', 
      nativeToScVal(toAddress, { type: 'address' }),
      nativeToScVal(amount, { type: 'i128' })
    ))
    .setTimeout(30)
    .build();
  
  return await kit.signAndSubmit(tx);
};
```

Mint submissions go through the OpenZeppelin Managed Relayer service so users do not need to configure local infrastructure for testnet funding.

---

### US-8: Compose Payment Email

**As a** user with USDC balance  
**I want to** compose an email to send USDC to someone  
**So that** they can receive funds without having a wallet

**Acceptance Criteria:**
- [ ] Send form displayed on dashboard with fields:
  - Sender email (required, validated)
  - Receiver email (required, validated)
  - Amount in USDC (required, positive number, <= balance)
  - Message (optional)
- [ ] Form CTA: "Send USDC"
- [ ] On submit, generate email content and display in modal popup
- [ ] Popup styled as email client preview showing:
  - From: `<sender_email>`
  - To: `<receiver_email>`
  - Cc: `check@mailspay.cc`
  - Subject: generated subject line
  - Body:
    ```
    [Optional user message]

    [Link: /receive?sender=<sender_email>&receiver=<receiver_email>&amount=<amount>&account=<sender_account>]

    ---BEGIN PAYMENT DATA---
    SENDER: <smart_account_address>
    AMOUNT: <amount_in_stroops>
    NONCE: <random_negative_number>
    ---END PAYMENT DATA---
    ```

---

### US-9: Send Payment Email

**As a** user viewing the email preview  
**I want to** send the email via my email client or copy it  
**So that** the recipient receives the payment link

**Acceptance Criteria:**
- [ ] Email preview popup has two action buttons:
  - **Send**: `<a href="mailto:...">` opens default email client with pre-filled To, Cc, Subject, Body
  - **Copy**: copies email body to clipboard
- [ ] On Copy click:
  - Toast warning: "Do not forget to include check@mailspay.cc in CC"
  - Toast info: "You will receive a confirmation email"
- [ ] Popup can be dismissed to return to dashboard

---

### US-10: Receive Payment Notification

**As a** payment recipient  
**I want to** land on a page from the email link  
**So that** I can create an account and receive the funds

**Acceptance Criteria:**
- [ ] Receive page accessible only via URL: `/receive?sender=<email>&receiver=<email>&amount=<amount>&account=<address>`
- [ ] Page not shown in header navigation
- [ ] Display message: "[Sender] is sending you [Amount] USDC"
- [ ] Display CTA: "Create account to receive"

---

### US-11: Create Receiver Account

**As a** payment recipient  
**I want to** create a smart account with one tap  
**So that** I can receive the USDC

**Acceptance Criteria:**
- [ ] Clicking "Create account to receive" triggers passkey creation via smart-account-kit
- [ ] On success, account stored in localStorage
- [ ] Generate reply email and display in modal popup styled as email client:
  - From: `<receiver_email>`
  - To: `<sender_email>`
  - Cc: `check@mailspay.cc`
  - Body: contains passkey public key and credential ID in required format

---

### US-12: Send Receiver Confirmation Email

**As a** payment recipient who created a passkey  
**I want to** send a confirmation email  
**So that** the backend can create my account and process the payment

**Acceptance Criteria:**
- [ ] Email preview popup has two action buttons:
  - **Send**: mailto link opens default email client
  - **Copy**: copies email body to clipboard
- [ ] On Copy click:
  - Toast warning: "Do not forget to include check@mailspay.cc in CC"
- [ ] Info banner below popup: "After sending this email (don't forget the CC!), you can close this window. You'll receive an email with your new wallet address once it's ready."

---

## Technical Specifications

### Data Models

**Note:** The smart-account-kit handles credential storage internally via `IndexedDBStorage`. The app only needs to cache balance data.

**App State (React Context):**
```ts
interface AppState {
  // Current connected wallet
  contractId: string | null;
  credentialId: string | null;
  senderEmail: string | null;  // Collected during account creation
  
  // Balance tracking
  balance: bigint | null;
  lastBalanceUpdate: number | null;
  
  // UI state
  isConnecting: boolean;
  isCreatingWallet: boolean;
}
```

**Sender Email Storage (localStorage):**
```ts
// Key: "emailpay:senderEmail"
// Value: "user@example.com"
```

**Balance Cache (localStorage):**
```ts
interface BalanceCache {
  [contractId: string]: {
    balance: string;      // Stored as string to preserve bigint
    updatedAt: number;    // Unix timestamp
  };
}
// Key: "emailpay:balanceCache"
```

**Email Payment Data Format (Sender → Service):**
```
---BEGIN PAYMENT DATA---
SENDER: <smart_account_address>
AMOUNT: <amount_in_stroops>
NONCE: <random_negative_integer>
---END PAYMENT DATA---
```

**Email Claim Data Format (Receiver → Service):**
```
---BEGIN CLAIM DATA---
PUBLIC_KEY: <secp256r1_public_key_hex_65_bytes>
CREDENTIAL_ID: <credential_id_hex>
---END CLAIM DATA---
```

**Note:** The receiver does NOT create a smart account. They only create a passkey. The backend uses the public key and credential ID to deploy the smart account on their behalf.

### Nonce Generation

The nonce is a random negative 64-bit integer used to prevent replay attacks:

```ts
const generateNonce = (): bigint => {
  // Generate random negative number between -2^63 and -1
  const randomBytes = crypto.getRandomValues(new Uint8Array(8));
  const view = new DataView(randomBytes.buffer);
  const value = view.getBigInt64(0, true);
  // Ensure negative
  return value > 0n ? -value : value === 0n ? -1n : value;
};
```

### UI Components

| Component | Description |
|-----------|-------------|
| `Header` | Logo, navigation (Home → `/`, Send → `/send`), current account indicator |
| `EmailPreviewPopup` | Modal styled as email compose window with From/To/Cc/Body |
| `Toast` | Notification component, top-right (desktop) / top-center (mobile), auto-dismiss 5s |
| `AccountSelector` | Dropdown for multiple accounts |
| `BalanceDisplay` | Shows USDC balance with loading/error states |
| `SendForm` | Form for receiver email, amount, message; triggers approval + email generation |
| `CreateWalletForm` | Email input field + "Create Wallet" button for new senders |
| `CreatePasskeyButton` | "Create passkey to receive" button for receivers (no email input needed) |

### Routing

| Route | Page | Access |
|-------|------|--------|
| `/` | Landing (About) | Header nav (Home) |
| `/send` | Send Dashboard | Header nav (Send) |
| `/receive` | Receiver | URL only (not in nav) |

### Validation Rules

- Email: valid email format (RFC 5322)
- Amount: positive number, max 2 decimal places, <= current balance
- All required fields must be non-empty

### Error Handling

- Display user-friendly error messages in toasts
- Handle network failures gracefully with retry option
- Validate inputs before submission
- Show loading states during async operations

### Loading & Error States Per Operation

| Operation | Loading State | Success State | Error State |
|-----------|--------------|---------------|-------------|
| Session restore | Spinner in header | Show send dashboard | Show create wallet CTA |
| Create wallet | "Creating wallet..." button disabled | Toast + show dashboard | Toast with error, retry button |
| Connect wallet | "Connecting..." button disabled | Toast + show dashboard | Toast with error |
| Fetch balance | Skeleton loader | Show balance | "Failed to load balance" + retry |
| Mint USDC | "Minting..." button disabled | Toast "Minted 100 USDC" | Toast with error, retry button |
| Approve gateway | "Approving payment..." button disabled | Proceed to email popup | Toast with error, retry button |
| Create passkey (receiver) | "Creating passkey..." button disabled | Show email popup | Toast with error, retry button |
| Generate email | Button spinner | Show email popup | Toast with error |
| Copy to clipboard | Brief "Copied!" | Toast "Copied to clipboard" | Toast "Failed to copy" |

### Mobile-First Requirements

- Design breakpoint: 375px minimum width
- Touch targets: minimum 44px
- Responsive layout scales to desktop
- No horizontal scrolling
- Readable font sizes (min 16px for inputs to prevent iOS zoom)

### Accessibility Requirements

- All interactive elements keyboard accessible
- Focus states visible on all focusable elements
- ARIA labels on icon-only buttons
- Color contrast ratio minimum 4.5:1 for text
- Loading states announced to screen readers (`aria-live`)
- Form inputs have associated labels
- Error messages linked to inputs via `aria-describedby`

---

## Dependencies

```json
{
  "dependencies": {
    "@openzeppelin/smart-account-kit": "github:kalepail/stellar-contracts#kalepail-edit",
    "@stellar/stellar-sdk": "^13.0.0",
    "react": "^18.3.0",
    "react-dom": "^18.3.0",
    "react-router-dom": "^6.28.0"
  },
  "devDependencies": {
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.0",
    "autoprefixer": "^10.4.0",
    "postcss": "^8.4.0",
    "tailwindcss": "^3.4.0",
    "typescript": "^5.6.0",
    "vite": "^6.0.0"
  }
}
```

**Note:** The smart-account-kit is installed from the GitHub branch until it's published to npm and is tracked as a git submodule in `/vendor` for reproducible builds.

---

## File Structure

```
src/
├── config.ts                    # App configuration
├── main.tsx                     # Entry point
├── App.tsx                      # Root component with routing
├── components/
│   ├── Header.tsx               # Logo, nav, account indicator
│   ├── EmailPreviewPopup.tsx    # Modal styled as email compose
│   ├── Toast.tsx                # Notification component
│   ├── AccountSelector.tsx      # Dropdown for multiple accounts
│   ├── BalanceDisplay.tsx       # USDC balance with loading state
│   ├── SendForm.tsx             # Payment form (receiver email, amount, message)
│   ├── CreateWalletForm.tsx     # Email input + create wallet CTA (for senders)
│   └── CreatePasskeyButton.tsx  # Create passkey CTA (for receivers)
├── pages/
│   ├── Landing.tsx              # Landing/About page (/)
│   ├── Send.tsx                 # Dashboard / send form (/send)
│   └── Receive.tsx              # Claim payment page (/receive)
├── hooks/
│   ├── useSmartAccount.ts       # SmartAccountKit wrapper
│   ├── useBalance.ts            # Balance polling logic
│   ├── useBalanceCache.ts       # localStorage balance cache
│   ├── useApproval.ts           # Gateway approval logic
│   └── useToast.ts              # Toast notifications
├── lib/
│   ├── smartAccount.ts          # SmartAccountKit instance
│   ├── stellar.ts               # Stellar SDK helpers (balance, mint)
│   ├── email.ts                 # Email content generation
│   └── format.ts                # Address truncation, amount formatting
├── context/
│   └── WalletContext.tsx        # Global wallet state
└── types/
    └── index.ts                 # Shared type definitions
```

### Key Hook: useSmartAccount

```ts
// src/hooks/useSmartAccount.ts
import { kit } from '../lib/smartAccount';

export function useSmartAccount() {
  const [contractId, setContractId] = useState<string | null>(null);
  const [senderEmail, setSenderEmail] = useState<string | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  // Silent restore on mount
  useEffect(() => {
    const restore = async () => {
      setIsConnecting(true);
      try {
        const result = await kit.connectWallet();
        if (result) {
          setContractId(result.contractId);
        }
        // Restore email from localStorage
        const storedEmail = localStorage.getItem('emailpay:senderEmail');
        if (storedEmail) {
          setSenderEmail(storedEmail);
        }
      } catch (e) {
        // No session - not an error
      } finally {
        setIsConnecting(false);
      }
    };
    restore();
  }, []);

  const createWallet = async (email: string) => {
    setIsConnecting(true);
    setError(null);
    try {
      const { contractId, credentialId } = await kit.createWallet(
        config.appName,
        email,
        { autoSubmit: true }
      );
      setContractId(contractId);
      setSenderEmail(email);
      // Persist email for future sessions
      localStorage.setItem('emailpay:senderEmail', email);
      return { contractId, credentialId };
    } catch (e) {
      setError(e as Error);
      throw e;
    } finally {
      setIsConnecting(false);
    }
  };

  const disconnect = async () => {
    await kit.disconnect();
    setContractId(null);
    // Keep email in localStorage for potential reconnection
  };

  return {
    kit,
    contractId,
    senderEmail,
    isConnecting,
    error,
    isConnected: !!contractId,
    createWallet,
    disconnect,
  };
}
```

---

## Out of Scope

- Authentication beyond passkeys
- Transaction history view
- Multiple currency support
- Email sending (only mailto links and clipboard copy)
- Backend services (client-side only, relies on smart-account-kit's IndexedDB)
- Payment status tracking (user receives confirmation via email from backend)
- Receiver's smart account deployment (done by backend after receiving claim email)

---

## Edge Cases & Decisions

### Amount Handling
- **Display**: Always show 2-7 decimal places, trim trailing zeros
- **Input**: Accept up to 7 decimal places
- **Conversion**: 1 USDC = 10^7 stroops (10,000,000)
- **Minimum**: No minimum enforced (contract may have its own limits)
- **Maximum**: Cannot exceed current balance

### URL Parameter Validation (`/receive` page)
- Missing `sender` → show error "Invalid payment link"
- Missing `receiver` → show error "Invalid payment link"  
- Missing `amount` → show error "Invalid payment link"
- Invalid `amount` (negative, NaN) → show error "Invalid amount"
- Missing `account` → show error "Invalid payment link"

### Multiple Accounts
- User has account A in storage, visits URL with `?account=B`:
  - Call `kit.connectWallet({ contractId: B })`
  - If successful, B becomes the active account
  - A remains in storage for future selection

### Receiver Already Has Passkey
- If receiver visits `/receive` and already has a credential stored:
  - Offer to use existing passkey: "Use existing passkey?" 
  - Or create a new one for this payment
  - Using existing passkey skips the WebAuthn creation prompt

### Mailto URL Length Limits
- Body content in mailto URLs is limited (~2000 chars)
- Keep message concise
- If user's optional message is too long, truncate with warning

### WebAuthn Domain Binding
- Passkeys are domain-specific (security feature)
- Account created on `app.mailspay.cc` cannot be used on `test.mailspay.cc`
- Document this limitation for users
