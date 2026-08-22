"""Bounded structured logging for the Hank Python worker.

Writes single-line JSON records to stderr (stdout carries the JSON-RPC
transport). Lines are bounded and secret-like values are masked before the
line leaves the process; the Rust capture applies redaction again before
retention. Stdlib only; no env, no fs, no execution of log content — a log
line is data, never a command.
"""

from __future__ import annotations

import json
import sys
from typing import TextIO

MAX_LINE_CHARS = 2_048
MAX_MESSAGE_CHARS = 512
TRUNCATION_MARKER = "...[truncated]"
REDACTED = "[redacted]"
SECRET_KEYS = (
    "token",
    "secret",
    "password",
    "passwd",
    "api_key",
    "apikey",
    "authorization",
    "auth",
    "bearer",
    "credential",
    "private_key",
)


def _redact_secrets(text: str) -> str:
    tokens: list[str] = []
    mask_next = False
    for token in text.split():
        bare = token.strip(',".').lower()
        if mask_next:
            tokens.append(REDACTED)
            mask_next = bare in SECRET_KEYS
            continue
        for separator in ("=", ":"):
            if separator in token:
                key_part, value_part = token.split(separator, 1)
                if key_part.strip(',".').lower() in SECRET_KEYS:
                    if value_part:
                        tokens.append(key_part + separator + REDACTED)
                    else:
                        tokens.append(token)
                        mask_next = True
                    break
        else:
            if bare in SECRET_KEYS:
                tokens.append(token)
                mask_next = True
            else:
                tokens.append(token)
    return " ".join(tokens)


def sanitize(message: str) -> str:
    """Strips control/ANSI characters, neutralizes path traversal, masks
    secret-like values and truncates to the bounded size."""
    cleaned = "".join(" " if ord(character) < 0x20 or ord(character) == 0x7F else character for character in message)
    cleaned = cleaned.replace("..", "_")
    cleaned = _redact_secrets(cleaned)
    if len(cleaned) > MAX_MESSAGE_CHARS:
        cleaned = cleaned[:MAX_MESSAGE_CHARS] + TRUNCATION_MARKER
    return cleaned[:MAX_LINE_CHARS]


def log(level: str, message: str, stream: TextIO | None = None) -> None:
    """Emits one bounded single-line JSON record to stderr."""
    target = stream if stream is not None else sys.stderr
    record = {
        "level": level if level in {"debug", "info", "warn", "error"} else "info",
        "message": sanitize(message),
    }
    line = json.dumps(record, separators=(",", ":"))
    target.write(line[:MAX_LINE_CHARS] + "\n")
    target.flush()
