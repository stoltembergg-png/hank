# Group chat event contract

O contrato `GroupChatEvent` carrega project, group, session, agent e trace IDs,
status, kind, texto e sequência. `GroupChatStore` aceita apenas eventos do
project/session ativos, em ordem, até o limite de mensagens.

Estados `pending`, `denied` e `terminated` são dados explícitos; `terminated`
fecha o store. Texto é limitado e `renderGroupChatText` escapa markup. A UI não
acessa DB, provider ou tool e não interpreta texto como instrução.
