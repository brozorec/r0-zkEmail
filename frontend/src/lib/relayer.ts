// OpenZeppelin Relayer Service Integration
// For submitting transactions via the managed relayer

import { xdr } from '@stellar/stellar-sdk';

/**
 * Relayer API endpoint
 */
const RELAYER_ENDPOINT = 'https://channels.openzeppelin.com/testnet';

/**
 * Relayer API key (should be set via environment variable)
 * TODO: Configure this via .env file
 */
const RELAYER_API_KEY = import.meta.env.VITE_RELAYER_API_KEY || '';

/**
 * Request format for the relayer service
 */
interface RelayerParams {
  /** Base64-encoded XDR of the HostFunction */
  func: string;
  /** Array of Base64-encoded XDR SorobanAuthorizationEntry objects */
  auth: string[];
}

interface RelayerRequest {
  params: RelayerParams;
}

/**
 * Response format from the relayer service
 */
interface TransactionData {
  hash: string;
  status: string;
  transaction_id: string;
}

interface RelayerResponse {
  success: boolean;
  data?: TransactionData;
  error?: any;
}

/**
 * Submit a transaction via the OpenZeppelin relayer service
 *
 * @param hostFunction - The Soroban host function to invoke (as XDR)
 * @param authEntries - Array of authorization entries (as XDR)
 * @returns Transaction result with hash and status
 */
export async function submitViaRelayer(
  hostFunction: xdr.HostFunction,
  authEntries: xdr.SorobanAuthorizationEntry[]
): Promise<TransactionData> {
  if (!RELAYER_API_KEY) {
    throw new Error('Relayer API key not configured. Set VITE_RELAYER_API_KEY environment variable.');
  }

  // Encode function and auth entries to base64 XDR
  const funcBase64 = hostFunction.toXDR('base64');
  const authBase64 = authEntries.map(entry => entry.toXDR('base64'));

  const request: RelayerRequest = {
    params: {
      func: funcBase64,
      auth: authBase64,
    },
  };

  // Make the API call with timeout
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), 60000); // 60s timeout for relayer

  try {
    const response = await fetch(RELAYER_ENDPOINT, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${RELAYER_API_KEY}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
      signal: controller.signal,
    });

    clearTimeout(timeoutId);

    if (!response.ok) {
      // Provide more specific error messages based on status code
      if (response.status === 401) {
        throw new Error('Invalid relayer API key. Please check your configuration.');
      } else if (response.status === 403) {
        throw new Error('Access forbidden. Please verify your relayer API key has the correct permissions.');
      } else if (response.status === 429) {
        throw new Error('Rate limit exceeded. Please try again later.');
      } else if (response.status >= 500) {
        throw new Error('Relayer service is currently unavailable. Please try again later.');
      }
      throw new Error(`Relayer API error: ${response.status} ${response.statusText}`);
    }

    const result: RelayerResponse = await response.json();

    if (!result.success || !result.data) {
      const errorMessage = result.error ?
        (typeof result.error === 'string' ? result.error : JSON.stringify(result.error)) :
        'Unknown error';
      throw new Error(`Transaction submission failed: ${errorMessage}`);
    }

    console.log('✅ Transaction submitted via relayer:', {
      hash: result.data.hash,
      status: result.data.status,
      transactionId: result.data.transaction_id,
    });

    return result.data;
  } catch (error) {
    clearTimeout(timeoutId);

    // Handle network and timeout errors
    if (error instanceof Error) {
      if (error.name === 'AbortError') {
        throw new Error('Request timed out. The relayer is taking too long to respond.');
      }
      if (error.message.includes('NetworkError') || error.message.includes('Failed to fetch')) {
        throw new Error('Network error: Unable to connect to relayer service. Please check your connection.');
      }
    }

    // Rethrow the error if it's already a custom error message
    throw error;
  }
}

/**
 * Submit a signed transaction XDR via the relayer
 * This is a simplified wrapper for cases where you have a fully signed transaction
 *
 * @param signedTxXdr - Base64-encoded signed transaction XDR
 * @returns Transaction result
 */
export async function submitSignedTransaction(_signedTxXdr: string): Promise<TransactionData> {
  if (!RELAYER_API_KEY) {
    throw new Error('Relayer API key not configured. Set VITE_RELAYER_API_KEY environment variable.');
  }

  // For signed transactions, we need to extract the host function and auth entries
  // This is a simplified version - adjust based on your transaction structure
  // const tx = xdr.Transaction.fromXDR(signedTxXdr, 'base64');

  // TODO: Extract host function and auth entries from the transaction
  // This depends on the transaction structure from Smart Account Kit
  throw new Error('Direct signed transaction submission not yet implemented. Use submitViaRelayer() with host function and auth entries.');
}

/**
 * Check relayer service health
 */
export async function checkRelayerHealth(): Promise<boolean> {
  try {
    const response = await fetch(`${RELAYER_ENDPOINT}/health`, {
      method: 'GET',
    });
    return response.ok;
  } catch (error) {
    console.error('Relayer health check failed:', error);
    return false;
  }
}
