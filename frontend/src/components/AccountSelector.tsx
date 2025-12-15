// Account selector dropdown for multiple accounts

import { useState } from 'react';
import { truncateAddress } from '../lib/format';

interface Account {
  contractId: string;
  email?: string;
  lastUsed?: number;
}

interface AccountSelectorProps {
  accounts: Account[];
  selectedAccount: string | null;
  onSelectAccount: (contractId: string) => void;
}

export default function AccountSelector({
  accounts,
  selectedAccount,
  onSelectAccount,
}: AccountSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);

  if (accounts.length <= 1) {
    return null;
  }

  const selected = accounts.find((a) => a.contractId === selectedAccount);

  return (
    <div className="relative">
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center space-x-2 px-4 py-2 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
        aria-haspopup="listbox"
        aria-expanded={isOpen}
      >
        <div className="text-left">
          {selected?.email && (
            <div className="text-xs text-gray-500">{selected.email}</div>
          )}
          <div className="text-sm font-mono text-gray-900">
            {selected ? truncateAddress(selected.contractId) : 'Select account'}
          </div>
        </div>
        <svg
          className={`w-4 h-4 text-gray-400 transition-transform ${
            isOpen ? 'transform rotate-180' : ''
          }`}
          fill="none"
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth="2"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path d="M19 9l-7 7-7-7"></path>
        </svg>
      </button>

      {isOpen && (
        <>
          {/* Backdrop */}
          <div
            className="fixed inset-0 z-10"
            onClick={() => setIsOpen(false)}
          />

          {/* Dropdown */}
          <div
            className="absolute top-full mt-2 w-64 bg-white border border-gray-200 rounded-lg shadow-lg z-20"
            role="listbox"
          >
            {accounts.map((account) => (
              <button
                key={account.contractId}
                onClick={() => {
                  onSelectAccount(account.contractId);
                  setIsOpen(false);
                }}
                className={`w-full text-left px-4 py-3 hover:bg-gray-50 transition-colors first:rounded-t-lg last:rounded-b-lg ${
                  account.contractId === selectedAccount
                    ? 'bg-blue-50 border-l-4 border-blue-600'
                    : ''
                }`}
                role="option"
                aria-selected={account.contractId === selectedAccount}
              >
                {account.email && (
                  <div className="text-xs text-gray-500 mb-1">
                    {account.email}
                  </div>
                )}
                <div className="text-sm font-mono text-gray-900">
                  {truncateAddress(account.contractId)}
                </div>
                {account.lastUsed && (
                  <div className="text-xs text-gray-400 mt-1">
                    Last used:{' '}
                    {new Date(account.lastUsed).toLocaleDateString()}
                  </div>
                )}
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
