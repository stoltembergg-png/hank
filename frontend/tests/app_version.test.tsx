import { type ReactNode } from 'react';
import { render, screen } from '@testing-library/react';
import { vi } from 'vitest';
import App from '../src/App';

vi.mock('../src/api/lifecycle', () => ({
  FRONTEND_READY_EVENT: 'hank:frontend-ready',
  FRONTEND_STARTUP_FAILED_EVENT: 'hank:frontend-startup-failed',
  notifyFrontendReady: vi.fn(() => Promise.resolve()),
  publishFrontendReady: vi.fn(),
  publishFrontendStartupFailure: vi.fn(),
}));

vi.mock('../src/components/ProductShell', () => ({
  ProductShell: ({ children }: { children: ReactNode }) => <main>{children}</main>,
}));

vi.mock('../src/components/ProjectList', () => ({
  ProjectList: () => <div data-testid="project-list" />,
}));

vi.mock('../src/components/ProjectDetailView', () => ({
  ProjectDetailView: () => <div data-testid="project-detail" />,
}));

test('displays the exact application release version', () => {
  render(<App />);

  expect(screen.getByText('Version: 1.0.0')).toBeInTheDocument();
});
