# Safe Markdown renderer contract

`frontend/src/chat/markdown/SafeMarkdown.tsx` renders a deliberately small Markdown subset for trusted UI structure but untrusted user/provider text. It never uses `dangerouslySetInnerHTML`, DOM parsing or executable HTML.

## Supported subset

- headings `#` through `####`;
- unordered and ordered lists;
- `**strong**`, `*emphasis*` and inline backticks;
- links with `http:` or `https:` schemes only.

Allowed external links receive `target="_blank"` and `rel="noreferrer noopener"`. Relative, `javascript:`, `data:`, `file:`, custom and malformed schemes render the label as plain text without an anchor.

Raw HTML is rendered as escaped React text and cannot create `script`, `img`, event-handler or arbitrary elements. The renderer does not log rejected content.

## Bounds and fallback

Input is bounded to `128 KiB` using UTF-8 byte length. Oversized content is deterministically truncated with `[conteúdo truncado]`. Empty content produces no DOM content. Unsupported Markdown syntax remains plain text; the next card owns fenced code block presentation.

## Chat integration

`ChatPage` renders assistant and user messages through `SafeMarkdown`, preserving the transport/session/generation security boundary. Message list bounds and stream validation remain in PR-091/PR-090.

## Tests

`frontend/tests/safe-markdown.test.tsx` covers semantic subset output, safe external links, hostile HTML/XSS, unsafe URL schemes, bounded large input and empty plain fallback.

## ONP mapping

- T-385 — Adicionar safe Markdown renderer [concluida]