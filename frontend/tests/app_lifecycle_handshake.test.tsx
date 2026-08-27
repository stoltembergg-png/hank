import { render, screen, waitFor } from '@testing-library/react';
import { vi } from 'vitest';
import App from '../src/App';

describe('App lifecycle handshake', () => {
  it('publishes readiness after the mounted component registers its listeners', async () => {
    const invoke = vi.fn(async (command: string) => {
      if (command === 'frontend_ready') return { stage: 'APPLICATION_READY' };
      if (command === 'list_projects') return { projects: [], total: 0 };
      throw new Error(`unexpected command: ${command}`);
    });
    const originalBridge = (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      configurable: true,
      value: { invoke },
    });

    try {
      render(<App />);
      await waitFor(() => expect(screen.getByText('ready')).toBeInTheDocument());
      expect(invoke).toHaveBeenCalledWith('frontend_ready');
    } finally {
      Object.defineProperty(window, '__TAURI_INTERNALS__', {
        configurable: true,
        value: originalBridge,
      });
    }
  });
});
