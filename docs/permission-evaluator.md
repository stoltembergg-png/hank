# Permission evaluator

O `PermissionEvaluator` é a decisão única antes da execução de uma tool.

## Precedência fail-closed

1. Projeto obrigatório.
2. Tool name/version obrigatórios.
3. Capability obrigatória.
4. `Deny` sempre nega.
5. Budget indisponível nega antes de pedir confirmação.
6. Efeitos read-only podem ser permitidos pela policy.
7. `Write`, `Execute`, `Credentials`, `Payment`, `InstallPackage` e `ForcePush` exigem confirmação quando a policy for `ask_once` ou `ask_every_time`.
8. `ask_once` é indexado por projeto, tool, versão e capability.
9. `ask_every_time` nunca usa aprovação anterior.

O cache é bounded a 1024 aprovações e vive apenas em memória. `clear_project` remove aprovações somente do projeto informado. Nenhum argumento, descrição ou payload é interpretado como policy.

A execução de handlers não pertence a este módulo: consumidores devem bloquear a chamada enquanto o resultado for `NeedsConfirmation` ou `Denied`.
