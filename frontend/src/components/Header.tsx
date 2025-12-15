// Header component with navigation and account indicator

import { Link } from 'react-router-dom';
import { useWallet } from '../context/WalletContext';
import AccountDropdown from './AccountDropdown';

export default function Header() {
  const { contractId, isConnecting } = useWallet();

  return (
    <header className="bg-white border-b border-gray-200">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex justify-between items-center h-16">
          {/* Logo and Navigation */}
          <div className="flex items-center space-x-8">
            <Link to="/" className="text-xl font-bold text-gray-900">
              Email Pay
            </Link>
          </div>

          {/* Account Indicator */}
          <div className="flex items-center min-w-[180px] justify-end">
            {isConnecting ? (
              <div className="text-sm text-gray-500">Connecting...</div>
            ) : contractId ? (
              <AccountDropdown />
            ) : null}
          </div>
        </div>
      </div>

      {/* Mobile Navigation */}
      <div className="md:hidden border-t border-gray-200">
        <nav className="px-4 py-2 space-x-4">
          <Link
            to="/"
            className="text-gray-600 hover:text-gray-900 text-sm font-medium"
          >
            Home
          </Link>
          <Link
            to="/send"
            className="text-gray-600 hover:text-gray-900 text-sm font-medium"
          >
            Send
          </Link>
        </nav>
      </div>
    </header>
  );
}
