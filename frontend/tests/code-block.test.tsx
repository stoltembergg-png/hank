import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { CodeBlock, MAX_CODE_BLOCK_BYTES } from '@/chat/code-block/CodeBlock';

describe('CodeBlock', () => {
  it('renders code as escaped plain text with an allowlisted language label', () => {
    const { container } = render(
      <CodeBlock language="typescript" code={'<script>alert(1)</script>\nconsole.log("safe")'} />,
    );
    expect(container.querySelector('script')).not.toBeInTheDocument();
    expect(container.querySelector('img')).not.toBeInTheDocument();
    expect(container.querySelector('code')).toHaveTextContent('<script>alert(1)</script>');
    expect(container.querySelector('code')).toHaveClass('language-typescript');
    expect(screen.getByText('typescript')).toBeInTheDocument();
    expect(container.querySelectorAll('a')).toHaveLength(0);
  });

  it('falls back unknown language and replaces ANSI/control characters', () => {
    const { container } = render(<CodeBlock language="rm -rf /" code={'one\u001b[31mtwo\u0000\nthree'} />);
    expect(container.querySelector('code')).toHaveClass('language-text');
    expect(screen.getByText('texto')).toBeInTheDocument();
    expect(container.textContent).not.toContain('\u001b');
    expect(container.textContent).not.toContain('\u0000');
  });

  it('bounds large code and reports deterministic truncation', () => {
    const { container } = render(<CodeBlock code={'x'.repeat(MAX_CODE_BLOCK_BYTES + 10_000)} />);
    expect(container.textContent?.length).toBeLessThanOrEqual(MAX_CODE_BLOCK_BYTES + 32);
    expect(container.textContent).toContain('código truncado');
  });

  it('copies only after explicit click and reports success/failure accessibly', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });
    render(<CodeBlock language="text" code="copy me" />);
    expect(writeText).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Copiar código' }));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith('copy me'));
    expect(screen.getByRole('status')).toHaveTextContent(/copiado/i);

    writeText.mockRejectedValueOnce(new Error('clipboard unavailable'));
    fireEvent.click(screen.getByRole('button', { name: 'Copiar código' }));
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent(/não foi possível copiar/i));
  });
});
