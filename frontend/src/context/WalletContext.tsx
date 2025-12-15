// Wallet Context for global wallet state management

import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  type ReactNode,
} from 'react';
import { kit } from '../lib/smartAccount';
import type { StoredAccount, WalletContextType } from '../types';
import { emitToast } from '../lib/toastBus';

const WalletContext = createContext<WalletContextType | undefined>(undefined);

const STORED_ACCOUNTS_KEY = 'emailpay:storedAccounts';
const SENDER_EMAIL_KEY = 'emailpay:senderEmail';
const isBrowser = typeof window !== 'undefined';

const sortAccounts = (accounts: StoredAccount[]) =>
  [...accounts].sort((a, b) => b.lastUsed - a.lastUsed);

export function WalletProvider({ children }: { children: ReactNode }) {
  const [contractId, setContractId] = useState<string | null>(null);
  const [credentialId, setCredentialId] = useState<string | null>(null);
  const [senderEmail, setSenderEmail] = useState<string | null>(null);
  const [balance, setBalance] = useState<bigint | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [isCreatingWallet, setIsCreatingWallet] = useState(false);
  const [storedAccounts, setStoredAccounts] = useState<StoredAccount[]>([]);
  const [storedAccountsError, setStoredAccountsError] = useState<string | null>(null);

  const persistStoredAccounts = useCallback((accounts: StoredAccount[]) => {
    if (!isBrowser) {
      return;
    }
    try {
      window.localStorage.setItem(STORED_ACCOUNTS_KEY, JSON.stringify(accounts));
      setStoredAccountsError(null);
    } catch (error) {
      console.error('Failed to save stored accounts:', error);
      setStoredAccountsError('Unable to save accounts');
      emitToast({ type: 'error', message: 'Unable to save accounts' });
    }
  }, []);

  const loadStoredAccounts = useCallback(() => {
    if (!isBrowser) {
      setStoredAccounts([]);
      return;
    }

    try {
      const raw = window.localStorage.getItem(STORED_ACCOUNTS_KEY);
      if (!raw) {
        setStoredAccounts([]);
        setStoredAccountsError(null);
        return;
      }

      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) {
        setStoredAccounts([]);
        setStoredAccountsError(null);
        return;
      }

      const sanitized: StoredAccount[] = parsed
        .filter((item) => item && typeof item.contractId === 'string')
        .map((item) => ({
          contractId: item.contractId,
          credentialId: item.credentialId ?? null,
          email: item.email ?? null,
          lastUsed: typeof item.lastUsed === 'number' ? item.lastUsed : Date.now(),
        }));

      const sorted = sortAccounts(sanitized);
      setStoredAccounts(sorted);
      setStoredAccountsError(null);
    } catch (error) {
      console.error('Failed to load stored accounts:', error);
      setStoredAccounts([]);
      setStoredAccountsError('Unable to load saved accounts');
      emitToast({ type: 'error', message: 'Unable to load saved accounts' });
    }
  }, []);

  const upsertStoredAccount = useCallback(
    (account: {
      contractId: string;
      credentialId?: string | null;
      email?: string | null;
      lastUsed?: number;
    }) => {
      setStoredAccounts((prev) => {
        const existing = prev.find((item) => item.contractId === account.contractId);
        const merged: StoredAccount = {
          contractId: account.contractId,
          credentialId: account.credentialId ?? existing?.credentialId ?? null,
          email: account.email ?? existing?.email ?? null,
          lastUsed: account.lastUsed ?? Date.now(),
        };

        const next = sortAccounts([
          merged,
          ...prev.filter((item) => item.contractId !== account.contractId),
        ]);

        persistStoredAccounts(next);
        return next;
      });
    },
    [persistStoredAccounts]
  );

  // Silent session restore on mount
  useEffect(() => {
    if (isBrowser) {
      loadStoredAccounts();
    }

    if (!isBrowser) {
      return;
    }

    const restore = async () => {
      setIsConnecting(true);
      try {
        const result = await kit.connectWallet();
        if (result) {
          setContractId(result.contractId);
          setCredentialId(result.credentialId);
        }

        // Restore email from localStorage
        const storedEmail = window.localStorage.getItem(SENDER_EMAIL_KEY);
        if (storedEmail) {
          setSenderEmail(storedEmail);
        }
      } catch (e) {
        console.error('Failed to restore wallet session:', e);
        // No session - not an error, user needs to create/connect
      } finally {
        setIsConnecting(false);
      }
    };

    restore();
  }, [loadStoredAccounts]);

  useEffect(() => {
    if (!isBrowser) {
      return;
    }

    const handleStorage = (event: StorageEvent) => {
      if (event.key === STORED_ACCOUNTS_KEY) {
        loadStoredAccounts();
      }

      if (event.key === SENDER_EMAIL_KEY && event.newValue) {
        setSenderEmail(event.newValue);
      }
    };

    window.addEventListener('storage', handleStorage);
    return () => window.removeEventListener('storage', handleStorage);
  }, [loadStoredAccounts]);

  const createWallet = async (email: string) => {
    setIsCreatingWallet(true);
    try {
      const result = await kit.createWallet(
        'Email Pay',
        email,
        { autoSubmit: true }
      );

      setContractId(result.contractId);
      setCredentialId(result.credentialId);
      setSenderEmail(email);

      // Persist email for future sessions
      if (isBrowser) {
        window.localStorage.setItem(SENDER_EMAIL_KEY, email);
      }

      upsertStoredAccount({
        contractId: result.contractId,
        credentialId: result.credentialId,
        email,
        lastUsed: Date.now(),
      });

      return result;
    } catch (e) {
      console.error('Failed to create wallet:', e);
      throw e;
    } finally {
      setIsCreatingWallet(false);
    }
  };

  const connectWallet = async (contractIdParam?: string) => {
    setIsConnecting(true);
    try {
      const matchingAccount = contractIdParam
        ? storedAccounts.find((account) => account.contractId === contractIdParam)
        : undefined;

      const result = await kit.connectWallet(
        contractIdParam
          ? {
              contractId: contractIdParam,
              ...(matchingAccount?.credentialId
                ? { credentialId: matchingAccount.credentialId }
                : {}),
            }
          : undefined
      );

      if (result) {
        setContractId(result.contractId);
        setCredentialId(result.credentialId);

        if (matchingAccount?.email) {
          setSenderEmail(matchingAccount.email);
          if (isBrowser) {
            window.localStorage.setItem(SENDER_EMAIL_KEY, matchingAccount.email);
          }
        }

        upsertStoredAccount({
          contractId: result.contractId,
          credentialId: result.credentialId,
          email: matchingAccount?.email ?? senderEmail,
          lastUsed: Date.now(),
        });
      }
    } catch (e) {
      console.error('Failed to connect wallet:', e);
      throw e;
    } finally {
      setIsConnecting(false);
    }
  };

  const disconnect = async () => {
    try {
      await kit.disconnect();
      setContractId(null);
      setCredentialId(null);
      setBalance(null);
      // Keep email in localStorage for potential reconnection
    } catch (e) {
      console.error('Failed to disconnect wallet:', e);
      throw e;
    }
  };

  const updateBalance = (newBalance: bigint) => {
    setBalance(newBalance);
  };

  const value: WalletContextType = {
    contractId,
    credentialId,
    senderEmail,
    balance,
    isConnecting,
    isCreatingWallet,
    storedAccounts,
    storedAccountsError,
    createWallet,
    connectWallet,
    disconnect,
    updateBalance,
  };

  return (
    <WalletContext.Provider value={value}>
      {children}
    </WalletContext.Provider>
  );
}

export function useWallet() {
  const context = useContext(WalletContext);
  if (context === undefined) {
    throw new Error('useWallet must be used within a WalletProvider');
  }
  return context;
}
