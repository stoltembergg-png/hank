import { useState } from 'react';
import './CodeBlock.css';

export const MAX_CODE_BLOCK_BYTES = 64 * 1024;
const TRUNCATION_MARKER = '\n… [código truncado]';

const LANGUAGE_LABELS = {
  text: 'texto',
  plaintext: 'texto',
  javascript: 'javascript',
  typescript: 'typescript',
  jsx: 'jsx',
  tsx: 'tsx',
  json: 'json',
  rust: 'rust',
  python: 'python',
  bash: 'bash',
  shell: 'shell',
  css: 'css',
  html: 'html',
  sql: 'sql',
  yaml: 'yaml',
  markdown: 'markdown',
} as const;

type Language = keyof typeof LANGUAGE_LABELS;
type CopyStatus = 'idle' | 'copied' | 'failed';

export function CodeBlock({ code, language }: { code: string; language?: string }) {
  const [copyStatus, setCopyStatus] = useState<CopyStatus>('idle');
  const safeCode = sanitizeCode(code);
  const safeLanguage = normalizeLanguage(language);
  const label = LANGUAGE_LABELS[safeLanguage];

  async function copyCode() {
    const clipboard = typeof navigator !== 'undefined' ? navigator.clipboard : undefined;
    if (!clipboard?.writeText) {
      setCopyStatus('failed');
      return;
    }
    try {
      await clipboard.writeText(safeCode);
      setCopyStatus('copied');
    } catch {
      setCopyStatus('failed');
    }
  }

  return (
    <figure className="safe-code-block" aria-label={`Bloco de código ${label}`}>
      <figcaption className="safe-code-block-header">
        <span>{label}</span>
        <button type="button" onClick={() => void copyCode()}>Copiar código</button>
      </figcaption>
      <pre><code className={`language-${safeLanguage}`}>{safeCode}</code></pre>
      <span className="safe-code-block-status" role="status" aria-live="polite">
        {copyStatus === 'copied' ? 'Código copiado.' : copyStatus === 'failed' ? 'Não foi possível copiar o código.' : ''}
      </span>
    </figure>
  );
}

function normalizeLanguage(language?: string): Language {
  const normalized = language?.trim().toLowerCase() as Language | undefined;
  return normalized && normalized in LANGUAGE_LABELS ? normalized : 'text';
}

function sanitizeCode(code: string): string {
  const encoder = new TextEncoder();
  const markerBytes = encoder.encode(TRUNCATION_MARKER).length;
  const bounded = boundCode(code, encoder, MAX_CODE_BLOCK_BYTES - markerBytes);
  const normalized: string[] = [];
  for (const character of bounded) {
    const point = character.codePointAt(0) ?? 0;
    if (character === '\n' || character === '\r' || character === '\t' || (point >= 0x20 && point !== 0x7f)) {
      normalized.push(character);
    } else {
      normalized.push('�');
    }
  }
  return bounded.length < code.length ? normalized.join('') + TRUNCATION_MARKER : normalized.join('');
}

function boundCode(code: string, encoder: TextEncoder, maxBytes: number): string {
  if (encoder.encode(code).length <= maxBytes) return code;
  let bytes = 0;
  let bounded = '';
  for (const character of code) {
    const characterBytes = encoder.encode(character).length;
    if (bytes + characterBytes > maxBytes) break;
    bounded += character;
    bytes += characterBytes;
  }
  return bounded;
}
