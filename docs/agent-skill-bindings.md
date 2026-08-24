# Agent skill bindings

Agent skill bindings are the explicit, project-scoped allow-list between an
active project Skill and an Agent. A project binding is required first; an
Agent cannot discover or import a Skill by binding it directly.

## Binding rules

- The request is bound to one project, Agent, Skill, exact active version,
  actor, capability, policy, bounded token budget and trace.
- The Agent must be active and belong to the requested project. The project
  binding must be enabled and its pinned version must exactly match the Agent
  request.
- Manifest capabilities must be allowed by both the binding policy and the
  current Agent policy; denied capabilities always win. A Skill budget or
  Agent request cannot exceed the Agent's request budget.
- Skills that require approval need an explicit approval identifier. Skill
  content is data and cannot change the binding policy or precedence.
- Precedence is deterministic: lower values load first, then the Skill ID;
  duplicate binding requests are idempotent.

## Lifecycle and loading

Disable and rollback revoke the Agent's active reference without rewriting
Skill history. Loading requires an enabled Agent binding and an enabled
project binding with the same exact version. The loader rechecks the Agent's
current lifecycle and capability policy, applies the stored token budget and
returns bounded untrusted data only.

Global Skills follow the project import boundary from
`project-skill-bindings.md`; an Agent binding never bypasses an explicit
project import or create cross-project visibility.

Every bind, disable and rollback emits the bounded `SkillBindingChanged`
event with project/Agent/Skill/version/precedence/budget/actor/approval/trace
metadata. Instructions and artifact content are never emitted.
