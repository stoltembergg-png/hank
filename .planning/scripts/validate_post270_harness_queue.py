#!/usr/bin/env python3
"""Integrity and dependency validator for the isolated post-270 Harness queue."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import sys

CONTRACT = Path('.planning/contracts/post-270-queue-extension-contract.json')


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def card_blocks(text: str) -> list[tuple[str, str]]:
    headings = list(re.finditer(r'^### (PR-\d{3}) — .+$', text, re.M))
    return [
        (
            heading.group(1),
            text[heading.start(): headings[i + 1].start() if i + 1 < len(headings) else len(text)],
        )
        for i, heading in enumerate(headings)
    ]


def main() -> int:
    contract = json.loads(CONTRACT.read_text())
    errors: list[str] = []

    for legacy in contract['legacy_queue']:
        path = Path(legacy['path'])
        if not path.is_file():
            errors.append(f"legacy queue missing: {path}")
        elif sha256(path) != legacy['sha256']:
            errors.append(f"legacy queue changed: {path}")

    extension = contract['extension_queue']
    queue = Path(extension['path'])
    if not queue.is_file():
        errors.append(f"extension queue missing: {queue}")
        blocks: list[tuple[str, str]] = []
    else:
        blocks = card_blocks(queue.read_text())

    ids = [card_id for card_id, _ in blocks]
    first = int(extension['first_id'].split('-')[1])
    last = int(extension['last_id'].split('-')[1])
    expected = [f'PR-{n:03d}' for n in range(first, last + 1)]
    if ids != expected:
        errors.append(f"extension IDs are not exactly {expected[0]}..{expected[-1]} in source order")
    if len(ids) != len(set(ids)):
        errors.append('duplicate extension card ID')

    deps: dict[str, list[str]] = {}
    allowed_ids = {*ids, extension['entry_dependency']}
    for card_id, block in blocks:
        for field in contract['required_fields']:
            if f'**{field}:**' not in block:
                errors.append(f'{card_id}: missing {field}')
        line = next((line for line in block.splitlines() if '**Dependências anteriores:**' in line), '')
        deps[card_id] = re.findall(r'PR-\d{3}', line)
        for dependency in deps[card_id]:
            if dependency not in allowed_ids:
                errors.append(f'{card_id}: unknown dependency {dependency}')

    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(card_id: str) -> bool:
        if card_id in visiting:
            return True
        if card_id in visited:
            return False
        visiting.add(card_id)
        for dependency in deps.get(card_id, []):
            if dependency in deps and visit(dependency):
                return True
        visiting.remove(card_id)
        visited.add(card_id)
        return False

    if any(visit(card_id) for card_id in ids):
        errors.append('combined extension dependency cycle detected')

    if errors:
        print('POST-270 QUEUE BLOCKED')
        print('\n'.join(f'- {error}' for error in errors))
        return 1
    print('POST-270 QUEUE PASS')
    print(
        f'legacy=PR-001..PR-270 immutable; extension={ids[0]}..{ids[-1]} '
        f'cards={len(ids)} dependencies={sum(map(len, deps.values()))} cycles=0'
    )
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
