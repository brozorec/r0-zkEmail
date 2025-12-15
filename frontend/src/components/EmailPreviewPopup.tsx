// Email preview popup styled as email compose window

import { useState } from 'react';
import type { EmailPreviewProps } from '../types';

export default function EmailPreviewPopup({
  from,
  to,
  cc,
  subject,
  body,
  onClose,
}: EmailPreviewProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(body);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      console.error('Failed to copy:', error);
    }
  };

  // Generate mailto URL
  const mailtoUrl = cc
    ? `mailto:${to}?cc=${cc}&subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`
    : `mailto:${to}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
      <div className="bg-white rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b">
          <h2 className="text-lg font-semibold text-gray-900">
            Email Preview
          </h2>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600"
            aria-label="Close"
          >
            <svg
              className="w-6 h-6"
              fill="none"
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth="2"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path d="M6 18L18 6M6 6l12 12"></path>
            </svg>
          </button>
        </div>

        {/* Email Fields */}
        <div className="p-4 space-y-3 border-b overflow-y-auto">
          <div className="flex text-sm">
            <span className="w-16 text-gray-500 flex-shrink-0">From:</span>
            <span className="text-gray-900">{from}</span>
          </div>
          <div className="flex text-sm">
            <span className="w-16 text-gray-500 flex-shrink-0">To:</span>
            <span className="text-gray-900">{to}</span>
          </div>
          {cc && (
            <div className="flex text-sm">
              <span className="w-16 text-gray-500 flex-shrink-0">Cc:</span>
              <span className="text-gray-900">{cc}</span>
            </div>
          )}
          <div className="flex text-sm">
            <span className="w-16 text-gray-500 flex-shrink-0">Subject:</span>
            <span className="text-gray-900 font-medium">{subject}</span>
          </div>
        </div>

        {/* Email Body */}
        <div className="p-4 flex-1 overflow-y-auto">
          <pre className="text-sm text-gray-700 whitespace-pre-wrap font-mono bg-gray-50 p-4 rounded">
            {body}
          </pre>
        </div>

        {/* Actions */}
        <div className="p-4 border-t flex flex-col sm:flex-row gap-3">
          <a
            href={mailtoUrl}
            className="flex-1 bg-blue-600 text-white px-6 py-2 rounded-lg hover:bg-blue-700 transition-colors text-center font-medium"
          >
            Send via Email Client
          </a>
          <button
            onClick={handleCopy}
            className="flex-1 bg-gray-100 text-gray-700 px-6 py-2 rounded-lg hover:bg-gray-200 transition-colors font-medium"
          >
            {copied ? 'Copied!' : 'Copy to Clipboard'}
          </button>
        </div>

        {/* Warning for CC */}
        {cc && (
          <div className="px-4 pb-4">
            <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-3 text-sm text-yellow-800">
              <strong>Important:</strong> Don't forget to include{' '}
              <span className="font-mono">{cc}</span> in CC when sending the
              email!
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
