import type { ReactNode } from 'react';
import './SafeMarkdown.css';

export const MAX_MARKDOWN_BYTES = 128 * 1024;
const TRUNCATION_MARKER = '\n\n[conteúdo truncado]';

type InlineToken = {
  label: string;
  url?: string;
  kind?: 'strong' | 'emphasis' | 'code';
};

export function SafeMarkdown({ source }: { source: string }) {
  const bounded = boundSource(source);
  if (!bounded) return null;
  return <div className="safe-markdown">{renderBlocks(bounded)}</div>;
}

function boundSource(source: string): string {
  const encoder = new TextEncoder();
  if (encoder.encode(source).length <= MAX_MARKDOWN_BYTES) return source;

  let bytes = 0;
  let bounded = '';
  for (const character of source) {
    const characterBytes = encoder.encode(character).length;
    if (bytes + characterBytes > MAX_MARKDOWN_BYTES - encoder.encode(TRUNCATION_MARKER).length) break;
    bounded += character;
    bytes += characterBytes;
  }
  return bounded + TRUNCATION_MARKER;
}

function renderBlocks(source: string): ReactNode[] {
  const lines = source.split(/\r?\n/u);
  const blocks: ReactNode[] = [];
  let index = 0;
  let key = 0;

  while (index < lines.length) {
    const line = lines[index];
    if (line.trim() === '') {
      index += 1;
      continue;
    }

    const heading = /^(#{1,4})\s+(.+)$/u.exec(line);
    if (heading) {
      const content = renderInline(heading[2], `heading-${key}`);
      const level = heading[1].length;
      if (level === 1) blocks.push(<h1 key={key}>{content}</h1>);
      else if (level === 2) blocks.push(<h2 key={key}>{content}</h2>);
      else if (level === 3) blocks.push(<h3 key={key}>{content}</h3>);
      else blocks.push(<h4 key={key}>{content}</h4>);
      key += 1;
      index += 1;
      continue;
    }

    const unordered = /^[-*]\s+(.+)$/u.exec(line);
    if (unordered) {
      const items: ReactNode[] = [];
      while (index < lines.length) {
        const item = /^[-*]\s+(.+)$/u.exec(lines[index]);
        if (!item) break;
        items.push(<li key={`${key}-${items.length}`}>{renderInline(item[1], `${key}-${items.length}`)}</li>);
        index += 1;
      }
      blocks.push(<ul key={key}>{items}</ul>);
      key += 1;
      continue;
    }

    const ordered = /^\d+[.]\s+(.+)$/u.exec(line);
    if (ordered) {
      const items: ReactNode[] = [];
      while (index < lines.length) {
        const item = /^\d+[.]\s+(.+)$/u.exec(lines[index]);
        if (!item) break;
        items.push(<li key={`${key}-${items.length}`}>{renderInline(item[1], `${key}-${items.length}`)}</li>);
        index += 1;
      }
      blocks.push(<ol key={key}>{items}</ol>);
      key += 1;
      continue;
    }

    const paragraph: string[] = [];
    while (index < lines.length && lines[index].trim() !== '' && !isBlockStart(lines[index])) {
      paragraph.push(lines[index]);
      index += 1;
    }
    blocks.push(<p key={key}>{renderInline(paragraph.join(' '), `paragraph-${key}`)}</p>);
    key += 1;
  }

  return blocks;
}

function isBlockStart(line: string): boolean {
  return /^(#{1,4})\s+.+$/u.test(line) || /^[-*]\s+.+$/u.test(line) || /^\d+[.]\s+.+$/u.test(line);
}

function renderInline(source: string, keyPrefix: string): ReactNode[] {
  const tokenPattern = /\[([^\]]+)\]\(([^)\s]+)\)|\*\*([^*]+)\*\*|\*([^*]+)\*|`([^`]+)`/gu;
  const nodes: ReactNode[] = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  let tokenIndex = 0;

  while ((match = tokenPattern.exec(source)) !== null) {
    if (match.index > lastIndex) nodes.push(source.slice(lastIndex, match.index));
    const key = `${keyPrefix}-${tokenIndex}`;
    const token: InlineToken = match[1]
      ? { label: match[1], url: match[2] }
      : match[3]
        ? { label: match[3], kind: 'strong' }
        : match[4]
          ? { label: match[4], kind: 'emphasis' }
          : { label: match[5], kind: 'code' };

    if (token.url && isSafeUrl(token.url)) {
      nodes.push(
        <a key={key} href={token.url} target="_blank" rel="noreferrer noopener">
          {token.label}
        </a>,
      );
    } else if (token.url) {
      nodes.push(token.label);
    } else if (token.kind === 'strong') {
      nodes.push(<strong key={key}>{token.label}</strong>);
    } else if (token.kind === 'emphasis') {
      nodes.push(<em key={key}>{token.label}</em>);
    } else {
      nodes.push(<code key={key}>{token.label}</code>);
    }
    lastIndex = tokenPattern.lastIndex;
    tokenIndex += 1;
  }

  if (lastIndex < source.length) nodes.push(source.slice(lastIndex));
  return nodes;
}

function isSafeUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return url.protocol === 'https:' || url.protocol === 'http:';
  } catch {
    return false;
  }
}
