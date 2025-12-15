// Smart Account Kit integration

import { SmartAccountKit, IndexedDBStorage } from 'smart-account-kit';
import { config } from '../config';

// Initialize the Smart Account Kit instance with IndexedDB storage
export const kit = new SmartAccountKit({
  rpcUrl: config.rpcUrl,
  networkPassphrase: config.networkPassphrase,
  accountWasmHash: config.accountWasmHash,
  webauthnVerifierAddress: config.webauthnVerifierAddress,
  storage: new IndexedDBStorage(),
});

// Export types for convenience
export type { CreateWalletResult, ConnectWalletResult, TransactionResult } from 'smart-account-kit';
