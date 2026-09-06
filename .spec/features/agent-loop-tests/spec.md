### US-2500 — Loop de agente determinístico e bounded

Como mantenedor, quero validar o ciclo provider→tool→próximo turno sem efeitos externos, para impedir loops infinitos, duplicação de tools e avanço após eventos inválidos.

#### AC-2501 — Sucesso e trace determinístico
Dado um roteiro sintético permitido.
Quando o loop executa a entrada.
Então termina em sucesso e produz o mesmo digest de trace para a mesma entrada.

#### AC-2502 — Idempotência de tool
Dado um pedido de tool repetido.
Quando o loop reprocessa o pedido.
Então registra replay sem cobrar novamente nem duplicar o efeito.

#### AC-2503 — Policy e limites
Dado tool negada, ciclo, profundidade ou budget excedido.
Quando o loop avalia o passo.
Então interrompe fail-closed sem avançar.

#### AC-2504 — Cancelamento e evento stale
Dado cancelamento ou evento ausente.
Quando o loop recebe o estado.
Então não executa novos passos e retorna estado terminal seguro.

#### AC-2505 — Limite de turns e política inválida
Dado limite inválido ou roteiro maior que o bound.
Quando o loop inicia.
Então rejeita a política ou termina no limite finito.

## Suposições
- O contrato é sintético e não afirma qualidade de modelo nem disponibilidade de provider.
- Efeitos reais de tools, rede, shell e credenciais permanecem fora do modelo.

## Perguntas em aberto
- Nenhuma.
