export const FRONTEND_READY_EVENT = 'hank:frontend-ready';
export const FRONTEND_STARTUP_FAILED_EVENT = 'hank:frontend-startup-failed';

export interface FrontendReadyResponse {
  stage: 'APPLICATION_READY';
}

type BridgeInvoker = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

interface LifecycleBridgeWindow {
  __TAURI_INTERNALS__?: {
    invoke?: BridgeInvoker;
  };
  __TAURI_INVOKE__?: BridgeInvoker;
}

export class FrontendBridgeUnavailableError extends Error {
  readonly code = 'FRONTEND_BRIDGE_UNAVAILABLE';

  constructor() {
    super('Frontend readiness requires the native desktop bridge');
    this.name = 'FrontendBridgeUnavailableError';
  }
}

function nativeInvoke(): BridgeInvoker | undefined {
  if (typeof window === 'undefined') return undefined;
  const bridge = window as unknown as LifecycleBridgeWindow;
  return bridge.__TAURI_INTERNALS__?.invoke ?? bridge.__TAURI_INVOKE__;
}

/**
 * Completes the native boot handshake. There is intentionally no browser or
 * timer-based fallback: without a real Tauri response the desktop app is not
 * ready.
 */
export async function notifyFrontendReady(): Promise<FrontendReadyResponse> {
  const invoke = nativeInvoke();
  if (typeof invoke !== 'function') throw new FrontendBridgeUnavailableError();
  return invoke<FrontendReadyResponse>('frontend_ready');
}

export function publishFrontendReady(): void {
  window.dispatchEvent(new Event(FRONTEND_READY_EVENT));
}

export function publishFrontendStartupFailure(error: unknown): void {
  const name = error instanceof Error ? error.name : 'UnknownError';
  window.dispatchEvent(new CustomEvent(FRONTEND_STARTUP_FAILED_EVENT, { detail: { name } }));
}
