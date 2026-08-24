export type SkillScope = 'project' | 'global';
export type SkillStatus = 'draft' | 'testing' | 'active' | 'deprecated' | 'archived' | 'blocked';
export type SkillCompatibility = 'initial' | 'compatible' | 'incompatible';

export interface SkillCapability {
  resource: string;
  action: string;
  scope?: string | null;
}

export interface SkillPolicySummary {
  requires_approval: boolean;
  allow_runtime_mutation: boolean;
  allow_instruction_override: boolean;
}

export interface SkillBudgetSummary {
  max_tokens: number;
  max_cost_micro_usd: number;
  max_parallel_invocations: number;
  max_wall_time_seconds: number;
  reset_period: string;
}

export interface SkillSourceSummary {
  kind: string;
  reference_digest: string;
}

export interface SkillVersionSummary {
  version: string;
  status: SkillStatus;
  compatibility: SkillCompatibility;
  content_hash: string;
  parent_version: string | null;
  created_at: string;
}

export interface SkillBindingSummary {
  project_id: string;
  scope: SkillScope;
  current_version: string;
  previous_version: string | null;
  import_reference: string | null;
  enabled: boolean;
  approval_id: string | null;
  trace_id: string;
  revision: number;
}

export interface SkillSummary {
  id: string;
  project_id: string | null;
  name: string;
  description: string;
  scope: SkillScope;
  status: SkillStatus;
  version: string;
  pinned_version: string | null;
  rollback_version: string | null;
  parent_version: string | null;
  compatibility: SkillCompatibility;
  content_hash: string;
  source: SkillSourceSummary;
  capabilities: SkillCapability[];
  policy: SkillPolicySummary;
  budget: SkillBudgetSummary;
  trace_id: string;
  revision: number;
  binding: SkillBindingSummary | null;
  versions: SkillVersionSummary[];
}

export interface ListSkillsInput {
  project_id: string;
  scope?: SkillScope;
  limit?: number;
  offset?: number;
}

export interface SkillListOutput {
  project_id: string;
  scope: SkillScope;
  skills: SkillSummary[];
  total: number;
  limit: number;
  offset: number;
  available?: boolean;
}

export interface SkillRollbackInput {
  project_id: string;
  skill_id: string;
  actor_id: string;
  trace_id: string;
  expected_revision: number;
  approval_id?: string | null;
  capability: 'skill.rollback';
  confirmed: true;
}
