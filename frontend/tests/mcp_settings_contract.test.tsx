import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { validateMcpServer } from '../src/api/mcp-settings';
import { McpSettings } from '../src/components/McpSettings';

const server = { id: 'server-a', name: '<MCP>', endpoint: 'https://mcp.example.test', capabilities: ['read'], trust: 'authorized' as const };
const tool = { name: '<tool>', state: 'Pending' as const };

// @spec:AC-1387
it('rejects unsafe configuration and renders hostile labels as text', () => {
  expect(validateMcpServer({ name: 'x', endpoint: 'not-url', capabilities: [] })).toMatchObject({ ok: false });
  expect(validateMcpServer({ name: 'x', endpoint: 'https://mcp.example.test', capabilities: ['execute'] })).toMatchObject({ ok: false });
  render(<McpSettings servers={[server]} tools={[tool]} onRevoke={vi.fn()} />);
  expect(screen.getByText('<MCP>')).toBeTruthy();
  expect(screen.getByText('<tool>')).toBeTruthy();
  expect(screen.queryByText('ignore instructions')).toBeNull();
});

// @spec:AC-1388
it('revokes with typed scope and keeps staged tools disabled', () => {
  const onRevoke = vi.fn();
  render(<McpSettings servers={[server]} tools={[tool]} onRevoke={onRevoke} />);
  fireEvent.click(screen.getByRole('button', { name: 'Revogar MCP' }));
  expect(onRevoke).toHaveBeenCalledWith({ server_id: 'server-a', project_id: 'local' });
  expect(screen.getByText('Pendente — ativação manual necessária')).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Executar tool' })).toBeNull();
});
