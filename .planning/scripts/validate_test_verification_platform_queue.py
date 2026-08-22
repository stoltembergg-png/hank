#!/usr/bin/env python3
"""Fail-closed integrity validator for the Test & Verification Platform queue."""
from __future__ import annotations
import hashlib
import json
from pathlib import Path
import re
import sys

CONTRACT = Path('.planning/contracts/test-verification-platform-queue-extension.json')
REQUIRED = ['ID','Categoria','Milestone','Título','Objetivo','Problema resolvido','Escopo','Não-escopo','Arquivos/crates prováveis','Contratos afetados','Dependências anteriores','Requisitos funcionais','NFRs','Critérios de aceite verificáveis','Testes unitários','Testes de integração','Testes de contrato','Testes negativos','E2E obrigatório quando aplicável','Performance quando aplicável','Verificações de segurança','Observabilidade','Evidência','Documentação','Rollback','Definition of Done','Condição para desbloquear a próxima PR']

def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main() -> int:
    c = json.loads(CONTRACT.read_text())
    errors = []
    for frozen in c['frozen_queues']:
        path = Path(frozen['path'])
        if not path.is_file() or digest(path) != frozen['sha256']:
            errors.append(f"frozen queue integrity mismatch: {path}")
    q = Path(c['extension_queue']['path'])
    text = q.read_text() if q.is_file() else ''
    heads = list(re.finditer(r'^### (PR-\d{3}) — .+$', text, re.M))
    ids = [m.group(1) for m in heads]
    first = int(c['extension_queue']['first_id'][3:]); last = int(c['extension_queue']['last_id'][3:])
    expected = [f'PR-{n:03d}' for n in range(first, last + 1)]
    if ids != expected: errors.append(f'IDs must be {expected[0]}..{expected[-1]}')
    known = {*ids, c['extension_queue']['entry_dependency']}
    deps = {}
    for index, head in enumerate(heads):
        block = text[head.start(): heads[index + 1].start() if index + 1 < len(heads) else len(text)]
        card = head.group(1)
        for field in REQUIRED:
            if f'**{field}:**' not in block: errors.append(f'{card}: missing {field}')
        line = next((line for line in block.splitlines() if '**Dependências anteriores:**' in line), '')
        deps[card] = re.findall(r'PR-\d{3}', line)
        errors.extend(f'{card}: unknown dependency {dep}' for dep in deps[card] if dep not in known)
    visiting, visited = set(), set()
    def visit(card):
        if card in visiting: return True
        if card in visited: return False
        visiting.add(card)
        bad = any(dep in deps and visit(dep) for dep in deps.get(card, []))
        visiting.remove(card); visited.add(card)
        return bad
    if any(visit(card) for card in deps): errors.append('dependency cycle detected')
    if errors:
        print('TEST PLATFORM QUEUE BLOCKED'); print('\n'.join(f'- {e}' for e in errors)); return 1
    print('TEST PLATFORM QUEUE PASS')
    print(f'frozen=PR-001..PR-345 extension={ids[0]}..{ids[-1]} cards={len(ids)} cycles=0')
    return 0
if __name__ == '__main__':
    raise SystemExit(main())
