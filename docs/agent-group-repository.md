# AgentGroup repository

O repository persiste o grupo como JSON não executável, com `project_id` na
chave primária composta, lifecycle, revisão otimista e timestamps. Toda leitura
exige projeto; create rejeita duplicata; archive rejeita revisão obsoleta e é
idempotente quando o estado já é `Archived`.

A migration não persiste conteúdo bruto de contexto além das referências
validadas pela entidade e não registra secrets.
