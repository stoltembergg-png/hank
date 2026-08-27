import { describe, expect, it, vi } from 'vitest';
import { FrontendBridgeUnavailableError, notifyFrontendReady } from '../src/api/lifecycle';

describe('desktop lifecycle handshake', () => {
  it('invokes the native readiness command and returns the application stage', async () => {
    const invoke = vi.fn().mockResolvedValue({ stage: 'APPLICATION_READY' });
    const originalWindow = globalThis.window;
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: { __TAURI_INTERNALS__: { invoke } },
    });

    try {
      await expect(notifyFrontendReady()).resolves.toEqual({ stage: 'APPLICATION_READY' });
      expect(invoke).toHaveBeenCalledWith('frontend_ready');
    } finally {
      Object.defineProperty(globalThis, 'window', { configurable: true, value: originalWindow });
    }
  });

  it('fails closed when the native bridge is unavailable', async () => {
    const originalWindow = globalThis.window;
    Object.defineProperty(globalThis, 'window', { configurable: true, value: {} });

    try {
      await expect(notifyFrontendReady()).rejects.toBeInstanceOf(FrontendBridgeUnavailableError);
    } finally {
      Object.defineProperty(globalThis, 'window', { configurable: true, value: originalWindow });
    }
  });
});
