import type { ToastMessage } from '../types';

type ToastPayload = Pick<ToastMessage, 'type' | 'message' | 'duration'>;

type ToastListener = (payload: ToastPayload) => void;

const listeners = new Set<ToastListener>();

/**
 * Subscribe to global toast events.
 * Returns an unsubscribe function to remove the listener.
 */
export function subscribeToToastBus(listener: ToastListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * Emit a toast event that any active toast containers can display.
 */
export function emitToast(payload: ToastPayload): void {
  listeners.forEach((listener) => listener(payload));
}
