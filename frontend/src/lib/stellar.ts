// Stellar SDK integration for balance fetching and USDC minting

import {
  Contract,
  Keypair,
  nativeToScVal,
  xdr,
  Address,
  rpc,
  TransactionBuilder,
  BASE_FEE,
} from '@stellar/stellar-sdk';
import { config } from '../config';
import { submitViaRelayer } from './relayer';

// Initialize Soroban RPC server for read operations
const server = new rpc.Server(config.rpcUrl);

// Deterministic dummy account for simulations (generated from a fixed seed)
const SIMULATION_SEED = 'emailpay-simulation-account-v1';
let simulationKeypair: Keypair | null = null;
let fundingPromise: Promise<void> | null = null;

/**
 * Get or create a deterministic keypair for simulations
 */
function getSimulationKeypair(): Keypair {
  if (!simulationKeypair) {
    // Generate deterministic keypair from seed
    const seedHash = new TextEncoder().encode(SIMULATION_SEED);
    // Pad/truncate to 32 bytes for keypair generation
    const seed = new Uint8Array(32);
    seed.set(seedHash.slice(0, Math.min(32, seedHash.length)));
    // Convert Uint8Array to Buffer for Stellar SDK
    const seedBuffer = Buffer.from(seed);
    simulationKeypair = Keypair.fromRawEd25519Seed(seedBuffer);
    console.log('🔑 Simulation account:', simulationKeypair.publicKey());
  }
  return simulationKeypair;
}

/**
 * Fund the simulation account using Friendbot (testnet only)
 * Uses a shared promise to ensure funding only happens once,
 * even with concurrent balance queries
 */
async function fundSimulationAccount(): Promise<void> {
  // If already funded or funding in progress, return the existing promise
  if (fundingPromise) {
    return fundingPromise;
  }

  // Create the funding promise
  fundingPromise = (async () => {
    const keypair = getSimulationKeypair();
    const publicKey = keypair.publicKey();

    try {
      console.log('💰 Funding simulation account via Friendbot:', publicKey);
      const friendbotUrl = `https://friendbot.stellar.org?addr=${encodeURIComponent(publicKey)}`;
      const response = await fetch(friendbotUrl);

      if (!response.ok) {
        throw new Error(`Friendbot request failed: ${response.status}`);
      }

      console.log('✅ Simulation account funded');
    } catch (error) {
      console.warn('⚠️ Failed to fund simulation account:', error);
      // Don't throw - account might already exist, funding will be retried on next call
      // Clear the promise so it can be retried if needed
      fundingPromise = null;
    }
  })();

  return fundingPromise;
}

/**
 * Fetches USDC balance for a given address using Soroban RPC
 * @param address - Smart account address (C-address)
 * @returns Balance in stroops (1 USDC = 10^7 stroops)
 */
export async function fetchBalance(address: string): Promise<bigint> {
  try {
    console.log('🔍 Fetching USDC balance for:', address);

    const contract = new Contract(config.usdcContractAddress);

    // Get deterministic simulation keypair
    const simulationKeypair = getSimulationKeypair();

    // Fund the simulation account if needed (only happens once)
    //await fundSimulationAccount();

    // Get the source account for building the transaction with timeout
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 30000); // 30s timeout

    try {
      const sourceAccount = await server.getAccount(simulationKeypair.publicKey());

      // Build a transaction to call balance(address)
      const transaction = new TransactionBuilder(sourceAccount, {
        fee: BASE_FEE,
        networkPassphrase: config.networkPassphrase,
      })
        .addOperation(
          contract.call('balance', nativeToScVal(address, { type: 'address' }))
        )
        .setTimeout(30)
        .build();

      // Simulate the transaction (read-only, not submitted)
      const response = await server.simulateTransaction(transaction);

      clearTimeout(timeoutId);

      if (rpc.Api.isSimulationSuccess(response)) {
        const resultValue = response.result?.retval;

        if (!resultValue) {
          console.warn('No balance value returned');
          return 0n;
        }

        // The balance is returned as i128
        // Parse the ScVal to get the balance
        const balance = scValToBigInt(resultValue);

        console.log('✅ Balance fetched:', {
          address: address.slice(0, 8) + '...',
          stroops: balance.toString(),
          usdc: Number(balance) / 10_000_000,
        });

        return balance;
      } else {
        console.error('Balance simulation failed:', response);
        // Return 0 if simulation fails (account might not exist yet)
        return 0n;
      }
    } finally {
      clearTimeout(timeoutId);
    }
  } catch (error) {
    console.error('❌ Failed to fetch balance:', error);

    // Handle specific error types
    if (error instanceof Error) {
      // Account not found - this is expected for new accounts
      if (error.message.includes('Account not found') ||
          error.message.includes('accountNotFound')) {
        console.log('Account not found, returning 0 balance');
        return 0n;
      }

      // Network errors
      if (error.message.includes('NetworkError') ||
          error.message.includes('Failed to fetch') ||
          error.name === 'AbortError') {
        throw new Error('Network error: Unable to connect to Stellar network. Please check your connection.');
      }

      // Timeout errors
      if (error.message.includes('timeout') || error.name === 'TimeoutError') {
        throw new Error('Request timed out. Please try again.');
      }
    }

    // For other errors, rethrow with generic message
    throw new Error('Failed to fetch balance. Please try again later.');
  }
}

/**
 * Convert a Soroban ScVal to BigInt
 * Handles i128 values returned from contract queries
 */
function scValToBigInt(scVal: xdr.ScVal): bigint {
  // Check if it's an i128 value
  if (scVal.switch().name === 'scvI128') {
    const i128 = scVal.i128();
    const hi = i128.hi();
    const lo = i128.lo();

    // Combine high and low parts into a bigint
    // i128 is stored as two i64 parts
    const value = (BigInt(hi.toString()) << 64n) | BigInt(lo.toString());

    return value;
  }

  // If it's a u128 value
  if (scVal.switch().name === 'scvU128') {
    const u128 = scVal.u128();
    const hi = u128.hi();
    const lo = u128.lo();

    const value = (BigInt(hi.toString()) << 64n) | BigInt(lo.toString());

    return value;
  }

  // If it's a simple integer
  if (scVal.switch().name === 'scvU64') {
    return BigInt(scVal.u64().toString());
  }

  if (scVal.switch().name === 'scvI64') {
    return BigInt(scVal.i64().toString());
  }

  console.warn('Unexpected ScVal type:', scVal.switch().name);
  return 0n;
}

/**
 * Build mint host function for relayer submission
 * @param toAddress - Recipient address
 * @param amount - Amount in stroops
 * @returns HostFunction XDR
 */
export function buildMintHostFunction(toAddress: string, amount: bigint): xdr.HostFunction {
  // Create the contract function arguments
  const args = [
    nativeToScVal(toAddress, { type: 'address' }),
    nativeToScVal(amount, { type: 'i128' })
  ];

  // Build the invoke contract host function
  return xdr.HostFunction.hostFunctionTypeInvokeContract(
    new xdr.InvokeContractArgs({
      contractAddress: new Address(config.usdcContractAddress).toScAddress(),
      functionName: 'mint',
      args,
    })
  );
}

/**
 * Mints testnet USDC to a given address via relayer
 * @param toAddress - Recipient address
 * @param amount - Amount in stroops
 * @param authEntries - Array of SorobanAuthorizationEntry from Smart Account Kit
 * @returns Transaction result from relayer
 */
export async function mintUsdc(
  toAddress: string,
  amount: bigint,
  authEntries: xdr.SorobanAuthorizationEntry[]
) {
  try {
    console.log('🪙 Minting USDC:', {
      to: toAddress,
      amount: amount.toString(),
      amountUsdc: Number(amount) / 10_000_000,
    });

    // Build the mint host function
    const hostFunction = buildMintHostFunction(toAddress, amount);

    // Submit via relayer
    const result = await submitViaRelayer(hostFunction, authEntries);

    console.log('✅ Mint successful:', result);

    return result;
  } catch (error) {
    console.error('❌ Mint failed:', error);
    throw error;
  }
}
