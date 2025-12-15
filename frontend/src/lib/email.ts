// Email content generation utilities

import { config } from '../config';
import { stroopsToUsdc } from './format';
import type { PaymentData, ClaimData } from '../types';

/**
 * Generates a random negative nonce for payment data
 * Used to prevent replay attacks
 */
export function generateNonce(): bigint {
  // Generate random negative number between -2^63 and -1
  const randomBytes = crypto.getRandomValues(new Uint8Array(8));
  const view = new DataView(randomBytes.buffer);
  const value = view.getBigInt64(0, true);

  // Ensure negative
  return value > 0n ? -value : value === 0n ? -1n : value;
}

/**
 * Generates payment email content for sender
 */
export function generatePaymentEmail(
  senderEmail: string,
  receiverEmail: string,
  amountStroops: bigint,
  smartAccountAddress: string,
  message?: string
): {
  from: string;
  to: string;
  cc: string;
  subject: string;
  body: string;
  mailtoUrl: string;
} {
  const nonce = generateNonce();
  const amountUsdc = stroopsToUsdc(amountStroops);

  // Generate claim link
  const claimUrl = `${window.location.origin}/receive?sender=${encodeURIComponent(
    senderEmail
  )}&receiver=${encodeURIComponent(receiverEmail)}&amount=${amountUsdc}&account=${encodeURIComponent(
    smartAccountAddress
  )}`;

  // Build email body
  const bodyParts = [];

  if (message) {
    bodyParts.push(message);
    bodyParts.push('');
  }

  bodyParts.push(`Click here to receive your payment: ${claimUrl}`);
  bodyParts.push('');
  bodyParts.push('---BEGIN PAYMENT DATA---');
  bodyParts.push(`SENDER: ${smartAccountAddress}`);
  bodyParts.push(`AMOUNT: ${amountStroops.toString()}`);
  bodyParts.push(`NONCE: ${nonce.toString()}`);
  bodyParts.push('---END PAYMENT DATA---');

  const body = bodyParts.join('\n');
  const subject = `${senderEmail.split('@')[0]} sent you ${amountUsdc} USDC`;

  // Generate mailto URL
  const mailtoUrl = `mailto:${receiverEmail}?cc=${config.checkEmail}&subject=${encodeURIComponent(
    subject
  )}&body=${encodeURIComponent(body)}`;

  return {
    from: senderEmail,
    to: receiverEmail,
    cc: config.checkEmail,
    subject,
    body,
    mailtoUrl,
  };
}

/**
 * Generates claim email content for receiver
 */
export function generateClaimEmail(
  receiverEmail: string,
  senderEmail: string,
  publicKey: string,
  credentialId: string
): {
  from: string;
  to: string;
  cc: string;
  subject: string;
  body: string;
  mailtoUrl: string;
} {
  const body = `---BEGIN CLAIM DATA---
PUBLIC_KEY: ${publicKey}
CREDENTIAL_ID: ${credentialId}
---END CLAIM DATA---`;

  const subject = `Re: Payment confirmation`;

  // Generate mailto URL
  const mailtoUrl = `mailto:${senderEmail}?cc=${config.checkEmail}&subject=${encodeURIComponent(
    subject
  )}&body=${encodeURIComponent(body)}`;

  return {
    from: receiverEmail,
    to: senderEmail,
    cc: config.checkEmail,
    subject,
    body,
    mailtoUrl,
  };
}

/**
 * Parses payment data from email body
 */
export function parsePaymentData(emailBody: string): PaymentData | null {
  const match = emailBody.match(
    /---BEGIN PAYMENT DATA---\s*SENDER:\s*(\S+)\s*AMOUNT:\s*(\S+)\s*NONCE:\s*(\S+)\s*---END PAYMENT DATA---/
  );

  if (!match) {
    return null;
  }

  return {
    sender: match[1],
    amount: match[2],
    nonce: match[3],
  };
}

/**
 * Parses claim data from email body
 */
export function parseClaimData(emailBody: string): ClaimData | null {
  const match = emailBody.match(
    /---BEGIN CLAIM DATA---\s*PUBLIC_KEY:\s*(\S+)\s*CREDENTIAL_ID:\s*(\S+)\s*---END CLAIM DATA---/
  );

  if (!match) {
    return null;
  }

  return {
    publicKey: match[1],
    credentialId: match[2],
  };
}
