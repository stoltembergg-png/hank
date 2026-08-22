#!/usr/bin/env python3
"""Planning validator for the isolated post-270 Harness queue."""
from pathlib import Path
import re
import sys

QUEUE = Path('.planning/queue/queue-271-345.md')
REQUIRED = [
    'ID', 'Categoria', 'Milestone', 'Título', 'Objetivo', 'Problema resolvido',
    'Escopo', 'Não-escopo', 'Arquivos/crates prováveis', 'Contratos afetados',
    'Dependências anteriores', 'Requisitos funcionais', 'NFRs',
    'Critérios de aceite verificáveis', 'Testes unitários', 'Testes de integração',
    'Testes de contrato', 'Testes negativos', 'E2E obrigatório quando aplicável',
    'Performance quando aplicável', 'Verificações de segurança', 'Observabilidade',
    'Evidência', 'Documentação', 'Rollback', 'Definition of Done',
    'Condição para desbloquear a próxima PR',
]


def main() -> int:
    text = QUEUE.read_text()
    headings = list(re.finditer(r'^### (PR-\d{3}) — .+$', text, re.M))
    cards = {}
    errors = []
    for i, heading in enumerate(headings):
        card_id = heading.group(1)
        block = text[heading.start(): headings[i + 1].start() if i + 1 < len(headings) else len(text)]
        if card_id in cards:
            errors.append(f'duplicate {card_id}')
        cards[card_id] = block
        for field in REQUIRED:
            if f'**{field}:**' not in block:
                errors.append(f'{card_id}: missing {field}')
    expected = [f'PR-{n:03d}' for n in range(271, 346)]
    if list(cards) != expected:
        errors.append('ids are not exactly PR-271..PR-345 in source order')
    deps = {}
    allowed = {'PR-270', *cards}
    for card_id, block in cards.items():
        line = next((line for line in block.splitlines() if '**Dependências anteriores:**' in line), '')
        found = re.findall(r'PR-\d{3}', line)
        deps[card_id] = found
        for dep in found:
            if dep not in allowed:
                errors.append(f'{card_id}: unknown dependency {dep}')
    visiting, visited = set(), set()
    def visit(card_id: str) -> bool:
        if card_id in visiting:
            return True
        if card_id in visited:
            return False
        visiting.add(card_id)
        for dep in deps[card_id]:
            if dep in cards and visit(dep):
                return True
        visiting.remove(card_id)
        visited.add(card_id)
        return False
    if any(visit(card) for card in cards):
        errors.append('dependency cycle detected')
    if errors:
        print('POST-270 QUEUE BLOCKED')
        print('\n'.join(f'- {error}' for error in errors))
        return 1
    print('POST-270 QUEUE PASS')
    print(f'cards={len(cards)} ids=PR-271..PR-345 dependencies={sum(map(len, deps.values()))} cycles=0')
    return 0

if __name__ == '__main__':
    raise SystemExit(main())
