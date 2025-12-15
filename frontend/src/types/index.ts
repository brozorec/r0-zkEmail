// Type definitions for Email Pay

export interface AppState {
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

export interface BalanceCache {
  [contractId: string]: {
    balance: string;      // Stored as string to preserve bigint
    updatedAt: number;    // Unix timestamp
  };
}

export interface StoredAccount {
  contractId: string;
  credentialId?: string | null;
  email?: string | null;
  lastUsed: number;
}

export interface EmailPreviewProps {
  from: string;
  to: string;
  cc?: string;
  subject: string;
  body: string;
  onClose: () => void;
}

export interface ToastMessage {
  id: string;
  type: 'success' | 'error' | 'warning' | 'info';
  message: string;
  duration?: number;
}

export interface WalletContextType {
  contractId: string | null;
  credentialId: string | null;
  senderEmail: string | null;
  balance: bigint | null;
  isConnecting: boolean;
  isCreatingWallet: boolean;
  storedAccounts: StoredAccount[];
  storedAccountsError: string | null;
  createWallet: (email: string) => Promise<{ contractId: string; credentialId: string }>;
  connectWallet: (contractId?: string) => Promise<void>;
  disconnect: () => Promise<void>;
  updateBalance: (balance: bigint) => void;
}

// Payment data format for sender email
export interface PaymentData {
  sender: string;      // Smart account address
  amount: string;      // Amount in stroops (as string)
  nonce: string;       // Random negative bigint (as string)
}

// Claim data format for receiver email
export interface ClaimData {
  publicKey: string;      // secp256r1 public key (65 bytes hex)
  credentialId: string;   // WebAuthn credential ID (hex)
}
