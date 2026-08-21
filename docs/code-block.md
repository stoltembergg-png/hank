# Safe code block renderer contract

`frontend/src/chat/code-block/CodeBlock.tsx` renders fenced Markdown code as escaped plain text. It is integrated into `SafeMarkdown` for fenced blocks and deliberately has no execution, shell, import, syntax-highlighter, file-write or sandbox capability.

## Rendering and safety

- Uses semantic `<figure>`, `<figcaption>`, `<pre>` and `<code>` elements.
- Language labels/classes come from a fixed allowlist; unknown or malicious language strings fall back to `text`/`texto`.
- Code is rendered as React text, so HTML/script/event-handler content cannot create DOM elements.
- ANSI escape and unsupported control characters are replaced with `�`; newline, carriage return and tab remain available for readable code.
- Code is bounded to `64 KiB` and receives a deterministic `[código truncado]` marker when oversized.
- URLs inside code are not auto-linked.

## Copy policy

The Copy button is the only clipboard path and requires an explicit user click. It writes the sanitized bounded code only; no clipboard read, automatic copy, execution or logging is performed. Success/failure is exposed through an accessible status region.

## Tests

`frontend/tests/code-block.test.tsx` covers escaped hostile code, language allowlist fallback, ANSI/control sanitization, size bounds, explicit clipboard success and clipboard failure. `frontend/tests/safe-markdown.test.tsx` verifies fenced blocks route to the code renderer.

## ONP mapping

- T-386 — Adicionar safe code-block renderer [concluida]