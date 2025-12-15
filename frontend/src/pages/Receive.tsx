// Receive page - Payment claim and receiver flow

import { useEffect, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useToast } from '../hooks/useToast';
import Header from '../components/Header';
import ToastContainer from '../components/Toast';
import CreatePasskeyButton from '../components/CreatePasskeyButton';
import { isValidEmail } from '../lib/format';

export default function Receive() {
  const [searchParams] = useSearchParams();
  const { toasts, removeToast, success, error: showError, warning } = useToast();

  const [validationError, setValidationError] = useState<string | null>(null);
  const [paymentData, setPaymentData] = useState<{
    sender: string;
    receiver: string;
    amount: string;
    account: string;
  } | null>(null);

  // Validate URL parameters on mount
  useEffect(() => {
    const sender = searchParams.get('sender');
    const receiver = searchParams.get('receiver');
    const amount = searchParams.get('amount');
    const account = searchParams.get('account');

    // Validate all required parameters
    if (!sender || !receiver || !amount || !account) {
      setValidationError('Invalid payment link - missing required parameters');
      return;
    }

    // Validate sender email
    if (!isValidEmail(sender)) {
      setValidationError('Invalid sender email address');
      return;
    }

    // Validate receiver email
    if (!isValidEmail(receiver)) {
      setValidationError('Invalid receiver email address');
      return;
    }

    // Validate amount
    const amountNum = parseFloat(amount);
    if (isNaN(amountNum) || amountNum <= 0) {
      setValidationError('Invalid payment amount');
      return;
    }

    // Validate account address (basic check)
    if (account.length < 10) {
      setValidationError('Invalid account address');
      return;
    }

    // All validation passed
    setPaymentData({ sender, receiver, amount, account });
  }, [searchParams]);

  const handlePasskeyCreated = () => {
    success('Passkey created successfully!');
  };

  const handlePasskeyError = (err: Error) => {
    if (err.message.includes('cancelled') || err.message.includes('canceled')) {
      warning('Passkey creation cancelled');
    } else if (err.message.includes('not supported')) {
      showError('Passkeys not supported on this device');
    } else {
      showError(err.message || 'Failed to create passkey');
    }
  };

  // Show error if validation failed
  if (validationError) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Header />
        <div className="max-w-2xl mx-auto px-4 py-12">
          <div className="bg-red-50 border border-red-200 rounded-lg p-6 text-center">
            <div className="text-5xl mb-4">⚠️</div>
            <h2 className="text-xl font-bold text-red-900 mb-2">
              Invalid Payment Link
            </h2>
            <p className="text-red-700">{validationError}</p>
            <p className="text-sm text-red-600 mt-4">
              Please check the link and try again, or contact the sender.
            </p>
          </div>
        </div>
      </div>
    );
  }

  // Show loading while validating
  if (!paymentData) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Header />
        <div className="max-w-2xl mx-auto px-4 py-12">
          <div className="text-center">
            <div className="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
            <p className="mt-4 text-gray-600">Loading payment details...</p>
          </div>
        </div>
      </div>
    );
  }

  // Show payment details and create passkey CTA
  return (
    <div className="min-h-screen bg-gray-50">
      <Header />
      <ToastContainer toasts={toasts} onClose={removeToast} />

      <div className="max-w-2xl mx-auto px-4 py-12">
        {/* Payment Notification */}
        <div className="bg-white border border-gray-200 rounded-lg p-8 mb-6">
          <div className="text-center mb-6">
            <div className="text-5xl mb-4">💰</div>
            <h1 className="text-3xl font-bold text-gray-900 mb-2">
              You've received a payment!
            </h1>
          </div>

          <div className="space-y-4 mb-8">
            <div className="flex justify-between items-center py-3 border-b">
              <span className="text-gray-600">From:</span>
              <span className="font-medium text-gray-900">
                {paymentData.sender}
              </span>
            </div>
            <div className="flex justify-between items-center py-3 border-b">
              <span className="text-gray-600">To:</span>
              <span className="font-medium text-gray-900">
                {paymentData.receiver}
              </span>
            </div>
            <div className="flex justify-between items-center py-3">
              <span className="text-gray-600">Amount:</span>
              <span className="text-2xl font-bold text-green-600">
                {paymentData.amount} USDC
              </span>
            </div>
          </div>

          <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6">
            <p className="text-sm text-blue-800">
              <strong>How it works:</strong> Create a secure passkey (using
              FaceID, TouchID, or PIN) to receive this payment. Your wallet will
              be created automatically after you send the confirmation email.
            </p>
          </div>

          <CreatePasskeyButton
            receiverEmail={paymentData.receiver}
            senderEmail={paymentData.sender}
            onSuccess={handlePasskeyCreated}
            onError={handlePasskeyError}
          />
        </div>

        {/* Info Box */}
        <div className="bg-gray-100 rounded-lg p-4 text-sm text-gray-700">
          <p className="font-medium mb-2">What happens next?</p>
          <ol className="list-decimal list-inside space-y-1">
            <li>You'll create a passkey (like unlocking your phone)</li>
            <li>
              You'll send a confirmation email to complete the wallet setup
            </li>
            <li>
              You'll receive an email with your wallet address once it's ready
            </li>
            <li>The USDC will be waiting in your new wallet!</li>
          </ol>
        </div>
      </div>
    </div>
  );
}
