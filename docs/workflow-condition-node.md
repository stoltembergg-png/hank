# ConditionNode evaluator

O `workflow_core::condition::ConditionExpression` é um evaluator declarativo e side-effect-free.

Subset suportado:

```text
$.campo == literal
$.campo != literal
$.campo > número
$.campo < número
```

Os literais são JSON primitivos. A expressão é limitada a 256 bytes e 16 segmentos de path.
Não há eval, chamadas de função, parênteses, rede, filesystem, memória ou mutação. Campo
inexistente, literal inválido, profundidade excessiva e incompatibilidade de tipos falham fechado.
