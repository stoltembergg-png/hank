import { SkillBudgetSummary, SkillPolicySummary, SkillStatus } from './skill';

export interface SkillEditorFile {
  path: string;
  role: string;
  content: string;
}

export interface SkillEditorDocument {
  project_id: string;
  skill_id: string;
  base_version: string;
  status: SkillStatus;
  revision: number;
  manifest_json: string;
  markdown: string;
  files: SkillEditorFile[];
  policy: SkillPolicySummary;
  budget: SkillBudgetSummary;
  trace_id: string;
  content_hash: string;
}

export interface SkillValidationDiagnostic {
  code: string;
  severity: string;
  line?: number | null;
}

export interface SkillValidationResult {
  valid: boolean;
  quarantined: boolean;
  diagnostics: SkillValidationDiagnostic[];
  errors: string[];
}

export interface SkillEditorLoadInput {
  project_id: string;
  skill_id: string;
  version?: string;
}

export interface SkillEditorValidateInput {
  project_id: string;
  skill_id: string;
  base_version: string;
  document: string;
  files?: SkillEditorFile[];
}

export interface SkillEditorSaveDraftInput extends SkillEditorValidateInput {
  actor_id: string;
  trace_id: string;
  expected_revision: number;
  budget: SkillBudgetSummary;
  policy: {
    allow: boolean;
    max_document_bytes: number;
  };
  capability: 'skill.edit';
  confirmed: true;
}

export interface SkillEditorDiscardInput {
  project_id: string;
  skill_id: string;
  actor_id: string;
  trace_id: string;
  expected_revision: number;
  version: string;
  capability: 'skill.discard';
  confirmed: true;
}

export interface SkillDraftResult {
  project_id: string;
  skill_id: string;
  version: string;
  status: SkillStatus;
  content_hash: string;
  changed: boolean;
  quarantined: boolean;
  revision: number;
}
