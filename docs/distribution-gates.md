# Distribution gates evaluator

## O que esta fatia entrega

`tools/distribution-gates.mjs` agrega a evidência de release das milestones de hardening
(segurança, fuzz, load, recovery, provider, backup/migração/secret, signing, installer,
updater, rollback) em uma decisão única e determinística:

- `NO_GO` para qualquer categoria ausente, identidade divergente (repo/ref/sha/tree/channel),
  status não-success, campo malformado ou acima do limite;
- `ELIGIBLE` somente para o tuple all-success exato, com `authorized=false` (nunca publica);
- rejeita prosa de IA/reviewer (`aiApproval`) como entrada;
- `reportDigest` determinístico (sha1 hex-40) apenas quando elegível.

## O que não está provado

O avaliador é offline e não publica, assina nem acessa credenciais. A publicação protegida
e a decisão canary automatizada são trabalho posterior. Esta feature prova somente o
contrato do avaliador; a distribuição real permanece `NO_PROOF`.

## Verificação local

```bash
node --test test/distribution-gates.test.mjs
node tools/ci/run-onp-spec.mjs verify distribution-gates
```

O teste cobre ausência de categoria, tuple all-success, status não-success, identidade
errada, mismatch de channel, evidência malformada/oversized, rejeição de `aiApproval` e
reprodutibilidade do `reportDigest`.