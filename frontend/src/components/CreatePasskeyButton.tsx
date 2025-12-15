// Create passkey button for receivers

import { useState } from 'react';
import { kit } from '../lib/smartAccount';
import { generateClaimEmail } from '../lib/email';
import EmailPreviewPopup from './EmailPreviewPopup';

interface CreatePasskeyButtonProps {
  receiverEmail: string;
  senderEmail: string;
  onSuccess?: () => void;
  onError?: (error: Error) => void;
}

export default function CreatePasskeyButton({
  receiverEmail,
  senderEmail,
  onSuccess,
  onError,
}: CreatePasskeyButtonProps) {
  const [isCreating, setIsCreating] = useState(false);
  const [showEmailPreview, setShowEmailPreview] = useState(false);
  const [emailPreview, setEmailPreview] = useState<ReturnType<
    typeof generateClaimEmail
  > | null>(null);

  const handleCreatePasskey = async () => {
    setIsCreating(true);

    try {
      // Create passkey without deploying on-chain (autoSubmit: false)
      // The backend will deploy the account after receiving the claim email
      const result = await kit.createWallet(
        'Email Pay',
        receiverEmail,
        { autoSubmit: false }
      );

      // Extract public key and credential ID from the result
      // Convert public key Uint8Array to hex string (65 bytes = 130 hex chars)
      const publicKeyHex = Array.from(result.publicKey)
        .map(b => b.toString(16).padStart(2, '0'))
        .join('');

      const credentialId = result.credentialId;

      console.log('Passkey created:', { publicKey: publicKeyHex, credentialId, contractId: result.contractId });

      // Generate claim email
      const preview = generateClaimEmail(
        receiverEmail,
        senderEmail,
        publicKeyHex,
        credentialId
      );

      setEmailPreview(preview);
      setShowEmailPreview(true);
      onSuccess?.();
    } catch (error) {
      console.error('Failed to create passkey:', error);
      const err =
        error instanceof Error ? error : new Error('Failed to create passkey');
      onError?.(err);
    } finally {
      setIsCreating(false);
    }
  };

  return (
    <>
      <button
        onClick={handleCreatePasskey}
        disabled={isCreating}
        className={`w-full py-4 px-6 rounded-lg font-medium text-lg transition-colors ${
          isCreating
            ? 'bg-gray-300 text-gray-500 cursor-not-allowed'
            : 'bg-green-600 text-white hover:bg-green-700'
        }`}
      >
        {isCreating ? (
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
            Creating passkey...
          </span>
        ) : (
          'Create Passkey to Receive'
        )}
      </button>

      {/* Email Preview Modal with instructions */}
      {showEmailPreview && emailPreview && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
          <div className="bg-white rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] flex flex-col">
            <EmailPreviewPopup
              {...emailPreview}
              onClose={() => setShowEmailPreview(false)}
            />

            {/* Additional instructions banner */}
            <div className="px-4 pb-4">
              <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 text-sm">
                <p className="text-blue-900 font-medium mb-2">
                  Next Steps:
                </p>
                <ol className="list-decimal list-inside space-y-1 text-blue-800">
                  <li>
                    Send the email above (don't forget to include{' '}
                    <span className="font-mono">{emailPreview.cc}</span> in CC!)
                  </li>
                  <li>
                    Wait for a confirmation email with your wallet address
                  </li>
                  <li>You can close this window after sending</li>
                </ol>
              </div>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
