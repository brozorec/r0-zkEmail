// Balance polling hook with change notifications

import { useState, useEffect, useCallback, useRef } from 'react';
import { fetchBalance } from '../lib/stellar';
import { useBalanceCache } from './useBalanceCache';
import { stroopsToUsdc } from '../lib/format';

interface UseBalanceOptions {
  contractId: string | null;
  pollInterval?: number; // milliseconds, default 5000
  onBalanceChange?: (oldBalance: bigint, newBalance: bigint) => void;
}

interface UseBalanceReturn {
  balance: bigint | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
}

/**
 * Hook for polling balance with caching and change detection
 */
export function useBalance({
  contractId,
  pollInterval = 5000,
  onBalanceChange,
}: UseBalanceOptions): UseBalanceReturn {
  const [balance, setBalance] = useState<bigint | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const { getCachedBalance, setCachedBalance } = useBalanceCache();
  const previousBalanceRef = useRef<bigint | null>(null);
  const onBalanceChangeRef = useRef(onBalanceChange);
  const activeContractIdRef = useRef<string | null>(contractId);
  const suppressNextChangeRef = useRef(false);

  useEffect(() => {
    onBalanceChangeRef.current = onBalanceChange;
  }, [onBalanceChange]);

  const fetchAndUpdateBalance = useCallback(async () => {
    if (!contractId) {
      setBalance(null);
      previousBalanceRef.current = null;
      return;
    }

    setIsLoading(true);
    setError(null);

    const requestContractId = contractId;

    try {
      const newBalance = await fetchBalance(requestContractId);

      if (activeContractIdRef.current !== requestContractId) {
        return;
      }

      const previousBalance = previousBalanceRef.current;
      const shouldNotify =
        !suppressNextChangeRef.current &&
        previousBalance !== null &&
        previousBalance !== newBalance;

      if (suppressNextChangeRef.current) {
        suppressNextChangeRef.current = false;
      }

      if (shouldNotify) {
        onBalanceChangeRef.current?.(previousBalance as bigint, newBalance);
      }

      setBalance(newBalance);
      previousBalanceRef.current = newBalance;
      setCachedBalance(requestContractId, newBalance);
    } catch (err) {
      const error = err instanceof Error ? err : new Error('Failed to fetch balance');
      setError(error);
      console.error('Balance fetch error:', error);
    } finally {
      if (activeContractIdRef.current === requestContractId) {
        setIsLoading(false);
      }
    }
  }, [contractId, setCachedBalance]);

  // Initialize with cached balance
  useEffect(() => {
    activeContractIdRef.current = contractId;
    suppressNextChangeRef.current = true;

    if (!contractId) {
      setBalance(null);
      previousBalanceRef.current = null;
      return;
    }

    const cached = getCachedBalance(contractId);
    if (cached) {
      setBalance(cached.balance);
      previousBalanceRef.current = cached.balance;
    } else {
      setBalance(null);
      previousBalanceRef.current = null;
    }
  }, [contractId, getCachedBalance]);

  // Poll balance at interval
  useEffect(() => {
    if (!contractId) {
      return;
    }

    // Fetch immediately
    fetchAndUpdateBalance();

    // Set up polling
    const intervalId = setInterval(fetchAndUpdateBalance, pollInterval);

    return () => {
      clearInterval(intervalId);
    };
  }, [contractId, pollInterval, fetchAndUpdateBalance]);

  return {
    balance,
    isLoading,
    error,
    refetch: fetchAndUpdateBalance,
  };
}

/**
 * Helper function to generate balance change toast messages
 */
export function getBalanceChangeMessage(
  oldBalance: bigint,
  newBalance: bigint
): { type: 'success' | 'info'; message: string } {
  const diff = newBalance - oldBalance;
  const absDiff = diff < 0n ? -diff : diff;
  const amount = stroopsToUsdc(absDiff);

  if (diff > 0n) {
    return {
      type: 'success',
      message: `Account credited with ${amount} USDC`,
    };
  } else {
    return {
      type: 'info',
      message: `Sent ${amount} USDC`,
    };
  }
}
