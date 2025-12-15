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

  useEffect(() => {
    onBalanceChangeRef.current = onBalanceChange;
  }, [onBalanceChange]);

  const fetchAndUpdateBalance = useCallback(async () => {
    if (!contractId) {
      setBalance(null);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const newBalance = await fetchBalance(contractId);

      // Compare with previous balance
      const previousBalance = previousBalanceRef.current;
      if (previousBalance !== null && previousBalance !== newBalance) {
        // Balance changed, notify callback
        onBalanceChangeRef.current?.(previousBalance, newBalance);
      }

      // Update state and cache
      setBalance(newBalance);
      previousBalanceRef.current = newBalance;
      setCachedBalance(contractId, newBalance);
    } catch (err) {
      const error = err instanceof Error ? err : new Error('Failed to fetch balance');
      setError(error);
      console.error('Balance fetch error:', error);
    } finally {
      setIsLoading(false);
    }
  }, [contractId, setCachedBalance]);

  // Initialize with cached balance
  useEffect(() => {
    if (!contractId) {
      setBalance(null);
      previousBalanceRef.current = null;
      return;
    }

    const cached = getCachedBalance(contractId);
    if (cached) {
      setBalance(cached.balance);
      previousBalanceRef.current = cached.balance;
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
