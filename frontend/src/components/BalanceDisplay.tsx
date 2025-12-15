// Balance display component with loading state

import { stroopsToUsdc } from '../lib/format';

interface BalanceDisplayProps {
  balance: bigint | null;
  isLoading?: boolean;
  error?: Error | null;
  onRetry?: () => void;
}

export default function BalanceDisplay({
  balance,
  isLoading,
  error,
  onRetry,
}: BalanceDisplayProps) {
  if (error) {
    return (
      <div className="bg-red-50 border border-red-200 rounded-lg p-4">
        <p className="text-red-800 text-sm mb-2">Failed to load balance</p>
        {onRetry && (
          <button
            onClick={onRetry}
            className="text-red-600 hover:text-red-700 text-sm font-medium underline"
          >
            Retry
          </button>
        )}
      </div>
    );
  }

  if (isLoading && balance === null) {
    return (
      <div className="bg-white border border-gray-200 rounded-lg p-6">
        <div className="animate-pulse">
          <div className="h-4 bg-gray-200 rounded w-24 mb-2"></div>
          <div className="h-8 bg-gray-200 rounded w-32"></div>
        </div>
      </div>
    );
  }

  const displayBalance = balance !== null ? stroopsToUsdc(balance) : '0.00';

  return (
    <div className="bg-gradient-to-br from-blue-50 to-blue-100 border border-blue-200 rounded-lg p-6">
      <div className="text-sm text-blue-600 font-medium mb-1">
        USDC Balance
      </div>
      <div className="text-3xl font-bold text-blue-900">
        {displayBalance}
        <span className="text-xl text-blue-600 ml-1">USDC</span>
      </div>
    </div>
  );
}
