# Cycle detection

`CycleDetector` é um preflight read-only. Ele classifica self-loop, procura o
callee nas identidades ancestrais e rejeita ancestry incompleta ou cross-project
sem alterar o `InvocationGraph`.

A decisão é determinística, bounded pela cadeia registrada e não pode ser
substituída por task text, prompt, provider ou qualquer resultado de modelo.
