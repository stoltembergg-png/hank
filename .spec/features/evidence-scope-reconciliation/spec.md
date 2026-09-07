# Spec: contract evidence scope reconciliation

> feature: evidence-scope-reconciliation
> status: em-andamento

## Histórias

### US-3000 — Distinguir contrato validado de integração de produção

Como mantenedor, quero que cada card de hardening declare a fronteira executada e a prova ausente para que documentação, CI e release gates não promovam contratos sintéticos a integrações reais.

#### AC-3001 — Manifesto central de escopo
- **Dado** o manifesto de escopo de evidência
- **Quando** o contrato for validado
- **Então** ele contém exatamente PR-261, PR-266 e PR-267, cada uma com `contractStatus` `PASS` e `productionStatus` `NO_PROOF`.

#### AC-3002 — Fuzz sintético
- **Dado** o registro de PR-261
- **Quando** suas alegações forem lidas
- **Então** ele identifica os sete targets como sintéticos e declara que fuzzing do parser de produção não foi provado.

#### AC-3003 — Signing sintético
- **Dado** o registro de PR-266
- **Quando** suas alegações forem lidas
- **Então** ele identifica as chaves como efêmeras/sintéticas e declara que assinatura ou publicação real não foi provada.

#### AC-3004 — Installers contract-only
- **Dado** o registro de PR-267
- **Quando** sua matriz for lida
- **Então** ela declara `contract-only` e que instalação, launch e uninstall nativos por plataforma não foram provados.

#### AC-3005 — Identidade da execução
- **Dado** o runner de reconciliação
- **Quando** os contratos passarem
- **Então** o relatório registra o commit e a árvore Git exatos da execução, além de rejeitar fontes ausentes ou resultado TAP incompleto.

#### AC-3006 — Prova visual honesta
- **Dado** um relatório de contrato válido
- **Quando** a representação visual for gerada
- **Então** ela mostra PASS/NO_PROOF por card e contém um aviso explícito de que não é prova de produção.

## Fora de escopo

- Executar fuzzing contra parsers de produção.
- Usar chave privada real ou publicar artefato.
- Construir/executar installers nativos em cada sistema operacional.
- Produzir screenshot ou gravação de uma aplicação quando o contrato não executa a aplicação.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## DoD

- Manifesto, spec, tarefas, runner e testes permanecem alinhados.
- O runner produz evidência vinculada a commit/tree exatos.
- O workflow publica JSON, SVG, HTML e TAP como artefatos.
- O resumo do check contém o link da execução e o limite `NOT PRODUCTION PROOF`.
- O verify ONP e o audit passam sem promover produção não executada.
