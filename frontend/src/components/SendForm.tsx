// Payment form for composing USDC transfers

import { useState } from 'react';
import { useWallet } from '../context/WalletContext';
import { isValidEmail, isValidAmount, usdcToStroops, stroopsToUsdc } from '../lib/format';
import { generatePaymentEmail } from '../lib/email';
import EmailPreviewPopup from './EmailPreviewPopup';

interface SendFormProps {
  onSuccess?: () => void;
  onError?: (error: Error) => void;
}

export default function SendForm({ onSuccess, onError }: SendFormProps) {
  const { senderEmail, contractId, balance } = useWallet();

  const [receiverEmail, setReceiverEmail] = useState('');
  const [amount, setAmount] = useState('');
  const [message, setMessage] = useState('');
  const [errors, setErrors] = useState<{
    receiver?: string;
    amount?: string;
  }>({});
  const [showEmailPreview, setShowEmailPreview] = useState(false);
  const [emailPreview, setEmailPreview] = useState<ReturnType<
    typeof generatePaymentEmail
  > | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    const newErrors: typeof errors = {};

    // Validate receiver email
    if (!receiverEmail) {
      newErrors.receiver = 'Receiver email is required';
    } else if (!isValidEmail(receiverEmail)) {
      newErrors.receiver = 'Please enter a valid email address';
    }

    // Validate amount
    if (!amount) {
      newErrors.amount = 'Amount is required';
    } else if (!isValidAmount(amount)) {
      newErrors.amount = 'Please enter a valid amount (max 7 decimal places)';
    } else {
      const amountStroops = usdcToStroops(amount);
      if (balance !== null && amountStroops > balance) {
        newErrors.amount = `Insufficient balance (you have ${stroopsToUsdc(balance)} USDC)`;
      }
    }

    if (Object.keys(newErrors).length > 0) {
      setErrors(newErrors);
      return;
    }

    setErrors({});

    try {
      if (!senderEmail || !contractId) {
        throw new Error('Wallet not connected');
      }

      const amountStroops = usdcToStroops(amount);
      const preview = generatePaymentEmail(
        senderEmail,
        receiverEmail,
        amountStroops,
        contractId,
        message || undefined
      );

      setEmailPreview(preview);
      setShowEmailPreview(true);
      onSuccess?.();
    } catch (error) {
      console.error('Failed to generate payment email:', error);
      const err =
        error instanceof Error ? error : new Error('Failed to generate payment');
      onError?.(err);
    }
  };

  const maxBalance = balance !== null ? stroopsToUsdc(balance) : '0';

  return (
    <>
      <div className="bg-white border border-gray-200 rounded-lg p-6">
        <h2 className="text-xl font-bold text-gray-900 mb-4">Send USDC</h2>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Sender Email (read-only) */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              From
            </label>
            <input
              type="text"
              value={senderEmail || ''}
              disabled
              className="w-full px-4 py-2 border border-gray-300 rounded-lg bg-gray-50 text-gray-600"
              style={{ fontSize: '16px' }}
            />
          </div>

          {/* Receiver Email */}
          <div>
            <label
              htmlFor="receiver"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              To
            </label>
            <input
              type="email"
              id="receiver"
              value={receiverEmail}
              onChange={(e) => {
                setReceiverEmail(e.target.value);
                setErrors((prev) => ({ ...prev, receiver: undefined }));
              }}
              placeholder="recipient@example.com"
              className={`w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent ${
                errors.receiver ? 'border-red-500' : 'border-gray-300'
              }`}
              aria-invalid={!!errors.receiver}
              aria-describedby={errors.receiver ? 'receiver-error' : undefined}
              style={{ fontSize: '16px' }}
            />
            {errors.receiver && (
              <p id="receiver-error" className="mt-1 text-sm text-red-600">
                {errors.receiver}
              </p>
            )}
          </div>

          {/* Amount */}
          <div>
            <label
              htmlFor="amount"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              Amount (USDC)
            </label>
            <div className="relative">
              <input
                type="text"
                id="amount"
                value={amount}
                onChange={(e) => {
                  setAmount(e.target.value);
                  setErrors((prev) => ({ ...prev, amount: undefined }));
                }}
                placeholder="0.00"
                className={`w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent ${
                  errors.amount ? 'border-red-500' : 'border-gray-300'
                }`}
                aria-invalid={!!errors.amount}
                aria-describedby={errors.amount ? 'amount-error' : undefined}
                style={{ fontSize: '16px' }}
              />
              <button
                type="button"
                onClick={() => setAmount(maxBalance)}
                className="absolute right-2 top-1/2 transform -translate-y-1/2 text-xs text-blue-600 hover:text-blue-700 font-medium"
              >
                MAX
              </button>
            </div>
            {errors.amount && (
              <p id="amount-error" className="mt-1 text-sm text-red-600">
                {errors.amount}
              </p>
            )}
            <p className="mt-1 text-xs text-gray-500">
              Available: {maxBalance} USDC
            </p>
          </div>

          {/* Message (optional) */}
          <div>
            <label
              htmlFor="message"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              Message (optional)
            </label>
            <textarea
              id="message"
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              placeholder="Add a note to your payment..."
              rows={3}
              className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent resize-none"
              style={{ fontSize: '16px' }}
            />
          </div>

          {/* Submit Button */}
          <button
            type="submit"
            className="w-full bg-blue-600 text-white py-3 px-4 rounded-lg font-medium hover:bg-blue-700 transition-colors"
          >
            Generate Payment Email
          </button>
        </form>
      </div>

      {/* Email Preview Modal */}
      {showEmailPreview && emailPreview && (
        <EmailPreviewPopup
          {...emailPreview}
          onClose={() => setShowEmailPreview(false)}
        />
      )}
    </>
  );
}
