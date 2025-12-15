// Create wallet form for new senders

import { useState } from 'react';
import { useWallet } from '../context/WalletContext';
import { isValidEmail } from '../lib/format';

interface CreateWalletFormProps {
  onSuccess?: (contractId: string) => void;
  onError?: (error: Error) => void;
}

export default function CreateWalletForm({
  onSuccess,
  onError,
}: CreateWalletFormProps) {
  const [email, setEmail] = useState('');
  const [emailError, setEmailError] = useState('');
  const { createWallet, isCreatingWallet } = useWallet();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    // Validate email
    if (!email) {
      setEmailError('Email is required');
      return;
    }

    if (!isValidEmail(email)) {
      setEmailError('Please enter a valid email address');
      return;
    }

    setEmailError('');

    try {
      const result = await createWallet(email);
      onSuccess?.(result.contractId);
    } catch (error) {
      console.error('Failed to create wallet:', error);
      const err = error instanceof Error ? error : new Error('Failed to create wallet');
      onError?.(err);
    }
  };

  return (
    <div className="bg-white border border-gray-200 rounded-lg p-8 max-w-md mx-auto">
      <div className="text-center mb-6">
        <h2 className="text-2xl font-bold text-gray-900 mb-2">
          Create Your Wallet
        </h2>
        <p className="text-gray-600">
          Enter your email to create a secure wallet using your device's passkey
          (FaceID, TouchID, or PIN)
        </p>
      </div>

      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label
            htmlFor="email"
            className="block text-sm font-medium text-gray-700 mb-1"
          >
            Your Email Address
          </label>
          <input
            type="email"
            id="email"
            value={email}
            onChange={(e) => {
              setEmail(e.target.value);
              setEmailError('');
            }}
            placeholder="you@example.com"
            className={`w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent ${
              emailError ? 'border-red-500' : 'border-gray-300'
            }`}
            disabled={isCreatingWallet}
            aria-invalid={!!emailError}
            aria-describedby={emailError ? 'email-error' : undefined}
            style={{ fontSize: '16px' }} // Prevent iOS zoom
          />
          {emailError && (
            <p id="email-error" className="mt-1 text-sm text-red-600">
              {emailError}
            </p>
          )}
        </div>

        <button
          type="submit"
          disabled={isCreatingWallet}
          className={`w-full py-3 px-4 rounded-lg font-medium transition-colors ${
            isCreatingWallet
              ? 'bg-gray-300 text-gray-500 cursor-not-allowed'
              : 'bg-blue-600 text-white hover:bg-blue-700'
          }`}
        >
          {isCreatingWallet ? (
            <span className="flex items-center justify-center">
              <svg
                className="animate-spin h-5 w-5 mr-2"
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
              Creating wallet...
            </span>
          ) : (
            'Create Wallet'
          )}
        </button>
      </form>

      <div className="mt-6 text-xs text-gray-500 text-center">
        <p>
          Your wallet is secured with your device's passkey. No passwords needed.
        </p>
      </div>
    </div>
  );
}
