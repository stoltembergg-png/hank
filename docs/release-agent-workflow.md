# Release-agent workflow

`agent-core::release_agent_workflow` prepara somente um candidato declarativo de release. O candidato exige identidade exata de repository/commit/tree/policy, artefato com digest `sha256:` e CI correspondente.

Evidência divergente ou incompleta produz `NoGo` com razões bounded. Signing, provenance, publishing, updater rollout, merge e aprovação pertencem a ambiente protegido externo; `can_publish()` permanece falso e nenhum segredo entra no domínio.
