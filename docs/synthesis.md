# Synthesis mode

`SynthesisPolicy` fornece um fallback determinístico e bounded para resultados
já produzidos. Cada item preserva result/source identity e é incluído somente
quando permitido e pertencente ao mesmo projeto.

Resultados duplicados, negados ou cross-project ficam no trace com razão de
exclusão. Facts múltiplos permanecem marcados como conflito. Conteúdo de
instruction é emitido como `[data]`, nunca como autoridade. O limite de saída
é aplicado antes do retorno.

A síntese não chama provider, não executa tools, não agenda trabalho e não
escreve memória.
