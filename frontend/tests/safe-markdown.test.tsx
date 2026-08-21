import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { MAX_MARKDOWN_BYTES, SafeMarkdown } from '@/chat/markdown/SafeMarkdown';

describe('SafeMarkdown', () => {
  it('renders the supported subset with semantic elements and safe external links', () => {
    const { container } = render(
      <SafeMarkdown source={'# Title\n\n- one\n- two\n\n**bold** and *emphasis* with [docs](https://example.com/docs)'} />,
    );
    expect(container.querySelector('h1')).toHaveTextContent('Title');
    expect(container.querySelectorAll('ul li')).toHaveLength(2);
    expect(container.querySelector('strong')).toHaveTextContent('bold');
    expect(container.querySelector('em')).toHaveTextContent('emphasis');
    const link = container.querySelector('a');
    expect(link).toHaveAttribute('href', 'https://example.com/docs');
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', 'noreferrer noopener');
  });

  it('renders hostile HTML as text and rejects unsafe URL schemes', () => {
    const { container } = render(
      <SafeMarkdown source={'<script>alert(1)</script> <img src=x onerror=alert(1)>\n\n[unsafe](javascript:alert(1))\n[data](data:text/html,<script>)'} />,
    );
    expect(container.querySelector('script')).not.toBeInTheDocument();
    expect(container.querySelector('img')).not.toBeInTheDocument();
    expect(container.querySelectorAll('a')).toHaveLength(0);
    expect(container.textContent).not.toContain('javascript:');
    expect(container.textContent).not.toContain('data:text');
    expect(container.textContent).toContain('unsafe');
  });

  it('routes fenced blocks to the non-executable code renderer', () => {
    const { container } = render(
      <SafeMarkdown source={'```rust\nprintln!("<script>");\n```'} />,
    );
    expect(container.querySelector('pre code')).toHaveClass('language-rust');
    expect(container.querySelector('pre code')).toHaveTextContent('println!("<script>");');
    expect(container.querySelector('script')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Copiar código' })).toBeInTheDocument();
  });
  it('bounds large input and keeps a deterministic plain-text fallback', () => {
    const source = 'x'.repeat(MAX_MARKDOWN_BYTES + 10_000);
    const { container } = render(<SafeMarkdown source={source} />);
    expect(container.textContent?.length).toBeLessThanOrEqual(MAX_MARKDOWN_BYTES + 32);
    expect(container.textContent).toContain('conteúdo truncado');
    expect(render(<SafeMarkdown source="" />).container.textContent).toBe('');
  });
});
