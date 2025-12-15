// Send page - Dashboard and sender flow

import { useEffect, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useWallet } from '../context/WalletContext';
import { useBalance, getBalanceChangeMessage } from '../hooks/useBalance';
import { useToast } from '../hooks/useToast';
import Header from '../components/Header';
import ToastContainer from '../components/Toast';
import CreateWalletForm from '../components/CreateWalletForm';
import BalanceDisplay from '../components/BalanceDisplay';
import SendForm from '../components/SendForm';

export default function Send() {
  const [searchParams] = useSearchParams();
  const accountParam = searchParams.get('account');

  const { contractId, isConnecting, connectWallet, updateBalance } = useWallet();
  const { toasts, removeToast, success, error: showError, warning } = useToast();
  const [isMinting, setIsMinting] = useState(false);

  // Balance polling with change notifications
  const {
    balance,
    isLoading: isLoadingBalance,
    error: balanceError,
    refetch: refetchBalance,
  } = useBalance({
    contractId,
    pollInterval: 10000,
    onBalanceChange: (oldBalance, newBalance) => {
      const { type, message } = getBalanceChangeMessage(oldBalance, newBalance);
      if (type === 'success') {
        success(message);
      } else {
        showError(message);
      }
      updateBalance(newBalance);
    },
  });

  // Update wallet context with balance
  useEffect(() => {
    if (balance !== null) {
      updateBalance(balance);
    }
  }, [balance, updateBalance]);

  // Connect wallet on mount (silent restore or URL param)
  useEffect(() => {
    const connect = async () => {
      if (accountParam) {
        try {
          await connectWallet(accountParam);
          success('Connected to wallet');
        } catch (err) {
          showError('Failed to connect to specified account');
        }
      }
      // Silent restore is handled by WalletContext
    };

    connect();
  }, [accountParam]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleWalletCreated = () => {
    success('Account created successfully!');
  };

  const handleWalletError = (err: Error) => {
    if (err.message.includes('cancelled') || err.message.includes('canceled')) {
      warning('Wallet creation cancelled');
    } else if (err.message.includes('not supported')) {
      showError('Passkeys not supported on this device');
    } else {
      showError(err.message || 'Failed to create wallet');
    }
  };

  const handleMintUsdc = async () => {
    if (!contractId) {
      showError('Wallet not connected');
      return;
    }

    setIsMinting(true);
    try {
      // Dynamically import the relayer integration
      const { mintUsdcViaRelayer } = await import('../lib/smartAccountRelayer');

      // Mint 100 USDC (100 * 10^7 stroops)
      const mintAmount = 100_0000000n;

      console.log('🚀 Initiating mint transaction via relayer...');
      const result = await mintUsdcViaRelayer(contractId, mintAmount);

      success(`Minted 100 USDC! Transaction: ${result.hash.slice(0, 8)}...`);

      // Refetch balance to show the updated amount
      setTimeout(() => refetchBalance(), 2000);
    } catch (err) {
      console.error('Mint error:', err);
      const errorMessage = err instanceof Error ? err.message : 'Failed to mint USDC';

      // Provide user-friendly error messages
      if (errorMessage.includes('API key not configured')) {
        showError('Relayer not configured. Please set VITE_RELAYER_API_KEY in .env');
      } else if (errorMessage.includes('401')) {
        showError('Invalid relayer API key');
      } else if (errorMessage.includes('Auth entry extraction')) {
        showError('Smart Account signing integration pending');
      } else {
        showError(errorMessage);
      }
    } finally {
      setIsMinting(false);
    }
  };

  const handlePaymentGenerated = () => {
    // Payment email generated successfully
    // The EmailPreviewPopup will handle the rest
  };

  const handlePaymentError = (err: Error) => {
    showError(err.message || 'Failed to generate payment email');
  };

  // Show loading state while connecting
  if (isConnecting) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Header />
        <div className="max-w-4xl mx-auto px-4 py-12">
          <div className="text-center">
            <div className="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
            <p className="mt-4 text-gray-600">Connecting wallet...</p>
          </div>
        </div>
      </div>
    );
  }

  // Show create wallet form if no wallet connected
  if (!contractId) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Header />
        <ToastContainer toasts={toasts} onClose={removeToast} />
        <div className="max-w-4xl mx-auto px-4 py-12">
          <CreateWalletForm
            onSuccess={handleWalletCreated}
            onError={handleWalletError}
          />
        </div>
      </div>
    );
  }

  // Show dashboard with balance and send form
  return (
    <div className="min-h-screen bg-gray-50">
      <Header />
      <ToastContainer toasts={toasts} onClose={removeToast} />

      <div className="max-w-4xl mx-auto px-4 py-8">
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900 mb-2">
            Send USDC via Email
          </h1>
          <p className="text-gray-600">
            Send USDC to anyone's email. They don't need a wallet - we'll create
            one for them.
          </p>
        </div>

        <div className="space-y-6">
          {/* Balance Display */}
          <BalanceDisplay
            balance={balance}
            isLoading={isLoadingBalance}
            error={balanceError}
            onRetry={refetchBalance}
          />

          {/* Mint USDC Button (only show if balance is 0) */}
          {balance !== null && balance === 0n && (
            <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
              <p className="text-sm text-blue-800 mb-3">
                You don't have any USDC yet. Mint some testnet USDC to get
                started.
              </p>
              <button
                onClick={handleMintUsdc}
                disabled={isMinting}
                className={`px-6 py-2 rounded-lg font-medium transition-colors ${
                  isMinting
                    ? 'bg-gray-300 text-gray-500 cursor-not-allowed'
                    : 'bg-blue-600 text-white hover:bg-blue-700'
                }`}
              >
                {isMinting ? (
                  <span className="flex items-center">
                    <svg
                      className="animate-spin h-4 w-4 mr-2"
                      xmlns="http://www.w3.org/2000/svg"
                      fill="none"
                      viewBox="0 0 24 24"
                    >
                      <circle
                        className="opacity-25"
                        cx="12"
                        cy="12"
                        r="10"
                        stroke="currentColor"
                        strokeWidth="4"
                      ></circle>
                      <path
                        className="opacity-75"
                        fill="currentColor"
                        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                      ></path>
                    </svg>
                    Minting...
                  </span>
                ) : (
                  'Mint 100 USDC'
                )}
              </button>
            </div>
          )}

          {/* Send Form */}
          <SendForm
            onSuccess={handlePaymentGenerated}
            onError={handlePaymentError}
          />
        </div>
      </div>
    </div>
  );
}
