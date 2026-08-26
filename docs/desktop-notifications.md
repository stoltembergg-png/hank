# Desktop notifications

PR-203 introduces a pure, bounded notification policy in `agent-runtime`.

The policy accepts only project-scoped automation signals, emits explicit
severity, redacts untrusted title/body text, suppresses duplicate event IDs,
applies a bounded per-window delivery count, and constructs only validated
`hank://runs/<project>/<run>` deep links.

A policy não chama APIs OS diretamente; a integração desktop entrega decisões aprovadas através do adapter Tauri descrito abaixo.
## Integração Tauri 2

A entrega OS usa o plugin oficial `tauri-plugin-notification` **2.3.3**, do
upstream `tauri-apps/plugins-workspace`, com licença MIT OR Apache-2.0. A versão
é compatível com o Tauri 2.11.5 resolvido pelo desktop e com Rust 1.97.1 (mínimo
declarado pelo plugin: 1.77.2).

O plugin é registrado somente no bootstrap Tauri. O runtime não importa Tauri:
`TauriNotificationSink` adapta `PermissionState`, `NotificationWorker` e
`DeliveryOutcome`. A capability concede apenas `allow-is-permission-granted`,
`allow-permission-state`, `allow-request-permission` e `allow-notify`; o conjunto
`notification:default` não é usado.

No desktop, o upstream informa estado/permissão `Granted` e usa o backend de
notificações do sistema. Linux usa `notify-rust`; Windows usa o contexto
instalado para o `AppUserModelId` quando aplicável; macOS usa o identificador
do aplicativo fora do modo de desenvolvimento. `cargo run` não é prova
suficiente de distribuição Windows instalada; o smoke de distribuição deve usar
o artefato empacotado quando a infraestrutura oferecer essa etapa.

Erros de permission state ou entrega são registrados sem incluir título/corpo
bruto. `Denied`, `Unavailable`, erro do plugin e mutex envenenado retornam
fallback controlado e não interrompem scheduler/runtime. Deep links continuam
somente na decisão da policy; como a API desktop do plugin não fornece
navegação de clique necessária para esse contrato, nenhum link arbitrário é
enviado ao OS.
