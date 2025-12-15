// Formatting utilities for addresses and amounts

/**
 * Truncates a Stellar address for display
 * Example: "CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX" -> "CXXX...XXXX"
 */
export function truncateAddress(address: string, startChars: number = 6, endChars: number = 4): string {
  if (!address || address.length <= startChars + endChars) {
    return address;
  }
  return `${address.slice(0, startChars)}...${address.slice(-endChars)}`;
}

/**
 * Converts stroops (smallest unit) to USDC display format
 * 1 USDC = 10^7 stroops
 * Display: 2-7 decimal places, trim trailing zeros
 */
export function stroopsToUsdc(stroops: bigint): string {
  const usdc = Number(stroops) / 10_000_000;

  // Format to 7 decimal places, then trim trailing zeros
  const formatted = usdc.toFixed(7).replace(/\.?0+$/, '');

  // Ensure at least 2 decimal places for readability
  if (!formatted.includes('.')) {
    return `${formatted}.00`;
  }

  const decimalPlaces = formatted.split('.')[1]?.length || 0;
  if (decimalPlaces === 1) {
    return `${formatted}0`;
  }

  return formatted;
}

/**
 * Converts USDC display format to stroops
 * 1 USDC = 10^7 stroops
 */
export function usdcToStroops(usdc: number | string): bigint {
  const amount = typeof usdc === 'string' ? parseFloat(usdc) : usdc;

  if (isNaN(amount) || amount < 0) {
    throw new Error('Invalid USDC amount');
  }

  // Multiply by 10^7 and convert to bigint
  const stroops = Math.round(amount * 10_000_000);
  return BigInt(stroops);
}

/**
 * Validates email format (RFC 5322 simplified)
 */
export function isValidEmail(email: string): boolean {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  return emailRegex.test(email);
}

/**
 * Validates USDC amount
 * - Must be positive
 * - Max 7 decimal places
 */
export function isValidAmount(amount: string): boolean {
  const amountRegex = /^\d+(\.\d{1,7})?$/;
  const num = parseFloat(amount);
  return amountRegex.test(amount) && !isNaN(num) && num > 0;
}

/**
 * Formats a timestamp to a readable date string
 */
export function formatTimestamp(timestamp: number): string {
  return new Date(timestamp).toLocaleString();
}
