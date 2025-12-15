// Balance caching hook using localStorage

import { useCallback } from 'react';
import type { BalanceCache } from '../types';

const CACHE_KEY = 'emailpay:balanceCache';

/**
 * Hook for managing balance cache in localStorage
 */
export function useBalanceCache() {
  const getCache = useCallback((): BalanceCache => {
    try {
      const cached = localStorage.getItem(CACHE_KEY);
      return cached ? JSON.parse(cached) : {};
    } catch (error) {
      console.error('Failed to read balance cache:', error);
      return {};
    }
  }, []);

  const getCachedBalance = useCallback(
    (contractId: string): { balance: bigint; updatedAt: number } | null => {
      const cache = getCache();
      const cached = cache[contractId];

      if (!cached) {
        return null;
      }

      try {
        return {
          balance: BigInt(cached.balance),
          updatedAt: cached.updatedAt,
        };
      } catch (error) {
        console.error('Failed to parse cached balance:', error);
        return null;
      }
    },
    [getCache]
  );

  const setCachedBalance = useCallback(
    (contractId: string, balance: bigint): void => {
      try {
        const cache = getCache();
        cache[contractId] = {
          balance: balance.toString(),
          updatedAt: Date.now(),
        };
        localStorage.setItem(CACHE_KEY, JSON.stringify(cache));
      } catch (error) {
        console.error('Failed to cache balance:', error);
      }
    },
    [getCache]
  );

  const clearCache = useCallback((contractId?: string): void => {
    try {
      if (contractId) {
        // Clear specific contract's cache
        const cache = getCache();
        delete cache[contractId];
        localStorage.setItem(CACHE_KEY, JSON.stringify(cache));
      } else {
        // Clear all cache
        localStorage.removeItem(CACHE_KEY);
      }
    } catch (error) {
      console.error('Failed to clear balance cache:', error);
    }
  }, [getCache]);

  return {
    getCachedBalance,
    setCachedBalance,
    clearCache,
  };
}
