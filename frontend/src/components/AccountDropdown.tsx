// Dropdown allowing users to switch between stored smart accounts or create new ones

import { useEffect, useMemo, useRef, useState } from 'react';
import { useWallet } from '../context/WalletContext';
import { truncateAddress, isValidEmail } from '../lib/format';
import { emitToast } from '../lib/toastBus';

function formatLastUsed(timestamp: number) {
  const diff = Date.now() - timestamp;
  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;

  if (diff < minute) {
    return 'just now';
  }
  if (diff < hour) {
    const minutes = Math.floor(diff / minute);
    return `${minutes}m ago`;
  }
  if (diff < day) {
    const hours = Math.floor(diff / hour);
    return `${hours}h ago`;
  }
  return new Date(timestamp).toLocaleDateString();
}

export default function AccountDropdown() {
  const {
    contractId,
    storedAccounts,
    storedAccountsError,
    connectWallet,
    createWallet,
    senderEmail,
    isConnecting,
    isCreatingWallet,
  } = useWallet();

  const [isOpen, setIsOpen] = useState(false);
  const [focusedIndex, setFocusedIndex] = useState(0);
  const [isSwitchingTo, setIsSwitchingTo] = useState<string | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [newAccountEmail, setNewAccountEmail] = useState(senderEmail ?? '');
  const [emailError, setEmailError] = useState('');

  const triggerRef = useRef<HTMLButtonElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const sortedAccounts = storedAccounts;
  const hasAccounts = sortedAccounts.length > 0;
  const disabled = Boolean(storedAccountsError);

  const activeIndex = useMemo(() => {
    if (!hasAccounts) {
      return -1;
    }
    const index = sortedAccounts.findIndex((account) => account.contractId === contractId);
    return index === -1 ? 0 : index;
  }, [sortedAccounts, contractId, hasAccounts]);

  useEffect(() => {
    if (isOpen && hasAccounts) {
      setFocusedIndex(activeIndex);
      setTimeout(() => menuRef.current?.focus(), 0);
    }
  }, [isOpen, activeIndex, hasAccounts]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    const handleClick = (event: MouseEvent | TouchEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node) &&
        triggerRef.current &&
        !triggerRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setIsOpen(false);
        triggerRef.current?.focus();
      }
    };

    document.addEventListener('mousedown', handleClick);
    document.addEventListener('touchstart', handleClick);
    document.addEventListener('keydown', handleEscape);

    return () => {
      document.removeEventListener('mousedown', handleClick);
      document.removeEventListener('touchstart', handleClick);
      document.removeEventListener('keydown', handleEscape);
    };
  }, [isOpen]);

  useEffect(() => {
    if (!showCreateForm) {
      setNewAccountEmail(senderEmail ?? '');
      setEmailError('');
    }
  }, [senderEmail, showCreateForm]);

  const toggleDropdown = () => {
    if (disabled) {
      return;
    }
    setIsOpen((prev) => !prev);
  };

  const handleMenuKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (!hasAccounts) {
      return;
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      setFocusedIndex((prev) => (prev + 1) % sortedAccounts.length);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      setFocusedIndex((prev) => (prev - 1 + sortedAccounts.length) % sortedAccounts.length);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      const account = sortedAccounts[focusedIndex];
      if (account) {
        void handleSelectAccount(account.contractId);
      }
    }
  };

  const handleSelectAccount = async (targetContractId: string) => {
    if (targetContractId === contractId) {
      setIsOpen(false);
      return;
    }

    setIsSwitchingTo(targetContractId);
    try {
      await connectWallet(targetContractId);
      emitToast({ type: 'success', message: 'Welcome back' });
      setIsOpen(false);
    } catch (error) {
      console.error('Failed to switch account:', error);
      emitToast({ type: 'error', message: 'Failed to switch account' });
    } finally {
      setIsSwitchingTo(null);
    }
  };

  const handleCreateAccount = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!newAccountEmail) {
      setEmailError('Email is required');
      return;
    }

    if (!isValidEmail(newAccountEmail)) {
      setEmailError('Please enter a valid email');
      return;
    }

    setEmailError('');

    try {
      await createWallet(newAccountEmail);
      emitToast({ type: 'success', message: 'Account created' });
      setShowCreateForm(false);
      setIsOpen(false);
    } catch (err) {
      const error = err instanceof Error ? err : new Error('Failed to create wallet');
      console.error('Wallet creation error:', error);

      if (error.message.toLowerCase().includes('cancel')) {
        emitToast({ type: 'info', message: 'Wallet creation cancelled' });
      } else if (error.message.toLowerCase().includes('not supported')) {
        emitToast({ type: 'error', message: 'Passkeys not supported on this device' });
      } else {
        emitToast({ type: 'error', message: error.message || 'Failed to create wallet' });
      }
    }
  };

  const currentAccount = sortedAccounts.find((account) => account.contractId === contractId);
  const currentLabel = currentAccount
    ? truncateAddress(currentAccount.contractId)
    : contractId
      ? truncateAddress(contractId)
      : 'Select account';

  return (
    <div className="relative" ref={dropdownRef}>
      <button
        ref={triggerRef}
        type="button"
        onClick={toggleDropdown}
        disabled={disabled || isConnecting || isCreatingWallet}
        aria-haspopup="menu"
        aria-expanded={isOpen}
        className={`flex items-center space-x-3 px-4 py-2 border rounded-lg shadow-sm text-sm font-medium transition-colors ${
          disabled
            ? 'bg-gray-100 text-gray-400 border-gray-200 cursor-not-allowed'
            : 'bg-white text-gray-900 border-gray-300 hover:bg-gray-50'
        }`}
      >
        <div className="text-left">
          <div className="text-xs text-gray-500">
            {currentAccount?.email ?? 'Stored account'}
          </div>
          <div className="flex items-center space-x-2">
            <span className="font-mono">{currentLabel}</span>
            <span
              className={`inline-block w-2 h-2 rounded-full ${
                disabled ? 'bg-gray-300' : 'bg-green-500'
              }`}
              aria-hidden="true"
            />
          </div>
        </div>
        <svg
          className={`w-4 h-4 text-gray-400 transition-transform ${
            isOpen ? 'transform rotate-180' : ''
          }`}
          viewBox="0 0 20 20"
          fill="none"
          stroke="currentColor"
        >
          <path d="M6 8l4 4 4-4" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>

      {storedAccountsError && (
        <p className="mt-1 text-xs text-red-600">{storedAccountsError}</p>
      )}

      {isOpen && !disabled && (
        <>
          <div
            className="fixed inset-0 z-10"
            aria-hidden="true"
            onClick={() => setIsOpen(false)}
          />
          <div
            ref={menuRef}
            role="menu"
            tabIndex={-1}
            className="absolute right-0 mt-2 w-80 bg-white border border-gray-200 rounded-lg shadow-xl z-20 focus:outline-none"
            onKeyDown={handleMenuKeyDown}
          >
            <div className="px-4 py-3 border-b border-gray-100">
              <p className="text-sm font-semibold text-gray-900">Saved accounts</p>
              <p className="text-xs text-gray-500">Switch between your passkey wallets</p>
            </div>

            <div className="max-h-64 overflow-y-auto" role="none">
              {hasAccounts ? (
                sortedAccounts.map((account, index) => {
                  const isActive = account.contractId === contractId;
                  const isSwitching = isSwitchingTo === account.contractId;
                  return (
                    <button
                      key={account.contractId}
                      type="button"
                      role="menuitemradio"
                      aria-checked={isActive}
                      className={`w-full text-left px-4 py-3 transition-colors ${
                        isActive ? 'bg-blue-50' : 'hover:bg-gray-50'
                      }`}
                      onClick={() => void handleSelectAccount(account.contractId)}
                      onMouseEnter={() => setFocusedIndex(index)}
                    >
                      <div className="flex items-center justify-between">
                        <div>
                          {account.email && (
                            <p className="text-xs text-gray-500">{account.email}</p>
                          )}
                          <p className="text-sm font-mono text-gray-900">
                            {truncateAddress(account.contractId)}
                          </p>
                        </div>
                        <div className="flex items-center space-x-2 text-xs text-gray-500">
                          {isSwitching && (
                            <svg
                              className="animate-spin h-4 w-4 text-blue-600"
                              viewBox="0 0 24 24"
                            >
                              <circle
                                className="opacity-25"
                                cx="12"
                                cy="12"
                                r="10"
                                stroke="currentColor"
                                strokeWidth="4"
                              />
                              <path
                                className="opacity-75"
                                fill="currentColor"
                                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                              />
                            </svg>
                          )}
                          {isActive && <span className="text-blue-600 font-medium">Active</span>}
                        </div>
                      </div>
                      <p className="text-xs text-gray-400 mt-1">
                        Last used: {formatLastUsed(account.lastUsed)}
                      </p>
                    </button>
                  );
                })
              ) : (
                <div className="px-4 py-6 text-center text-sm text-gray-500">
                  No stored accounts yet. Create one below.
                </div>
              )}
            </div>

            {hasAccounts && sortedAccounts.length === 1 && (
              <div className="px-4 py-2 text-xs text-gray-500 border-t border-gray-100">
                Only one account saved
              </div>
            )}

            <div className="border-t border-gray-100">
              {showCreateForm ? (
                <form onSubmit={handleCreateAccount} className="px-4 py-3 space-y-2">
                  <label className="text-xs font-medium text-gray-600">
                    Email for new account
                    <input
                      type="email"
                      value={newAccountEmail}
                      onChange={(event) => {
                        setNewAccountEmail(event.target.value);
                        setEmailError('');
                      }}
                      className={`mt-1 w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent text-sm ${
                        emailError ? 'border-red-500' : 'border-gray-300'
                      }`}
                      placeholder="you@example.com"
                      autoFocus
                    />
                  </label>
                  {emailError && <p className="text-xs text-red-600">{emailError}</p>}
                  <div className="flex items-center justify-end space-x-2 text-sm">
                    <button
                      type="button"
                      className="px-3 py-1.5 rounded-lg text-gray-600 hover:text-gray-900"
                      onClick={() => setShowCreateForm(false)}
                    >
                      Cancel
                    </button>
                    <button
                      type="submit"
                      disabled={isCreatingWallet}
                      className="px-3 py-1.5 rounded-lg bg-blue-600 text-white hover:bg-blue-700 disabled:bg-blue-300 disabled:cursor-not-allowed"
                    >
                      {isCreatingWallet ? 'Creating...' : 'Create account'}
                    </button>
                  </div>
                </form>
              ) : (
                <button
                  type="button"
                  className="w-full px-4 py-3 text-sm font-medium text-blue-600 hover:bg-blue-50 text-left"
                  onClick={() => setShowCreateForm(true)}
                >
                  + Create new account
                </button>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
