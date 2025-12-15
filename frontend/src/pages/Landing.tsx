// Landing page - Marketing and onboarding

import { Link } from 'react-router-dom';
import Header from '../components/Header';

export default function Landing() {
  return (
    <div className="min-h-screen bg-gradient-to-b from-blue-50 to-white">
      <Header />

      {/* Hero Section */}
      <div className="max-w-6xl mx-auto px-4 py-16 sm:py-24">
        <div className="text-center">
          <h1 className="text-5xl sm:text-6xl font-bold text-gray-900 mb-6">
            Send USDC via Email
          </h1>
          <p className="text-xl sm:text-2xl text-gray-600 mb-8 max-w-3xl mx-auto">
            The easiest way to send cryptocurrency. No wallet needed for recipients.
            Just an email address.
          </p>

          <div className="flex flex-col sm:flex-row gap-4 justify-center mb-12">
            <Link
              to="/send"
              className="px-8 py-4 bg-blue-600 text-white text-lg font-semibold rounded-lg hover:bg-blue-700 transition-colors"
            >
              Send USDC Now
            </Link>
            <a
              href="#how-it-works"
              className="px-8 py-4 bg-white text-blue-600 text-lg font-semibold rounded-lg border-2 border-blue-600 hover:bg-blue-50 transition-colors"
            >
              Learn More
            </a>
          </div>

          {/* Trust Indicators */}
          <div className="flex flex-wrap justify-center items-center gap-8 text-sm text-gray-500">
            <div className="flex items-center gap-2">
              <span className="text-green-600 text-xl">✓</span>
              <span>Powered by Stellar</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-green-600 text-xl">✓</span>
              <span>Secure WebAuthn</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-green-600 text-xl">✓</span>
              <span>No wallet required</span>
            </div>
          </div>
        </div>
      </div>

      {/* Features Section */}
      <div className="bg-white py-16" id="how-it-works">
        <div className="max-w-6xl mx-auto px-4">
          <h2 className="text-3xl sm:text-4xl font-bold text-gray-900 text-center mb-12">
            How It Works
          </h2>

          <div className="grid md:grid-cols-3 gap-8">
            {/* Feature 1 */}
            <div className="text-center p-6">
              <div className="text-5xl mb-4">📧</div>
              <h3 className="text-xl font-bold text-gray-900 mb-3">
                Enter Email & Amount
              </h3>
              <p className="text-gray-600">
                Simply enter the recipient's email address and the amount of USDC
                you want to send. No need to know their wallet address.
              </p>
            </div>

            {/* Feature 2 */}
            <div className="text-center p-6">
              <div className="text-5xl mb-4">🔐</div>
              <h3 className="text-xl font-bold text-gray-900 mb-3">
                Secure Passkey
              </h3>
              <p className="text-gray-600">
                Recipients create a secure passkey (FaceID, TouchID, or PIN) to
                access their funds. No passwords to remember.
              </p>
            </div>

            {/* Feature 3 */}
            <div className="text-center p-6">
              <div className="text-5xl mb-4">💰</div>
              <h3 className="text-xl font-bold text-gray-900 mb-3">
                Instant Access
              </h3>
              <p className="text-gray-600">
                Once verified, the USDC is immediately available in their new
                wallet. Fast, secure, and simple.
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Benefits Section */}
      <div className="py-16 bg-gray-50">
        <div className="max-w-6xl mx-auto px-4">
          <h2 className="text-3xl sm:text-4xl font-bold text-gray-900 text-center mb-12">
            Why Email Pay?
          </h2>

          <div className="grid md:grid-cols-2 gap-8 max-w-4xl mx-auto">
            <div className="bg-white rounded-lg p-6 shadow-sm">
              <h3 className="text-lg font-bold text-gray-900 mb-2">
                No Wallet Needed
              </h3>
              <p className="text-gray-600">
                Recipients don't need an existing crypto wallet. We create one for
                them automatically using their email.
              </p>
            </div>

            <div className="bg-white rounded-lg p-6 shadow-sm">
              <h3 className="text-lg font-bold text-gray-900 mb-2">
                Secure & Private
              </h3>
              <p className="text-gray-600">
                Built on Stellar blockchain with WebAuthn passkeys. Your funds are
                protected by the same technology that secures your device.
              </p>
            </div>

            <div className="bg-white rounded-lg p-6 shadow-sm">
              <h3 className="text-lg font-bold text-gray-900 mb-2">
                Fast & Cheap
              </h3>
              <p className="text-gray-600">
                Transactions settle in seconds with minimal fees. Send any amount,
                big or small.
              </p>
            </div>

            <div className="bg-white rounded-lg p-6 shadow-sm">
              <h3 className="text-lg font-bold text-gray-900 mb-2">
                Easy Recovery
              </h3>
              <p className="text-gray-600">
                Your passkey is tied to your email and device. No seed phrases to
                write down or lose.
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* CTA Section */}
      <div className="py-16 bg-blue-600">
        <div className="max-w-4xl mx-auto px-4 text-center">
          <h2 className="text-3xl sm:text-4xl font-bold text-white mb-4">
            Ready to send your first payment?
          </h2>
          <p className="text-xl text-blue-100 mb-8">
            Get started in less than a minute. No signup required.
          </p>
          <Link
            to="/send"
            className="inline-block px-8 py-4 bg-white text-blue-600 text-lg font-semibold rounded-lg hover:bg-blue-50 transition-colors"
          >
            Send USDC Now
          </Link>
        </div>
      </div>

      {/* Footer */}
      <div className="bg-gray-900 text-gray-400 py-8">
        <div className="max-w-6xl mx-auto px-4 text-center">
          <p className="text-sm">
            Powered by Stellar blockchain and OpenZeppelin infrastructure
          </p>
          <p className="text-xs mt-2">
            Testnet only - For demonstration purposes
          </p>
        </div>
      </div>
    </div>
  );
}
