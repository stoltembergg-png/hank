export type MemoryType =
  | 'fact'
  | 'preference'
  | 'decision'
  | 'lesson'
  | 'project_context'
  | 'technical_context'
  | 'failure'
  | 'successful_pattern';

export type MemoryStatus = 'candidate' | 'approved' | 'rejected' | 'archived';

export type MemoryProvenance =
  | 'user_input'
  | 'agent_output'
  | 'tool_result'
  | 'skill_execution'
  | 'workflow_node'
  | 'external_import'
  | 'inferred';

export interface MemorySummary {
  id: string;
  project_id: string;
  agent_id?: string | null;
  memory_type: MemoryType;
  status: MemoryStatus;
  content: string;
  summary?: string | null;
  importance: number;
  provenance: MemoryProvenance;
  confidence: number;
  trace_id?: string | null;
  version?: number;
  created_at: string;
  updated_at: string;
}

export interface ListMemoriesInput {
  project_id: string;
  agent_id?: string;
  status?: MemoryStatus;
  memory_type?: MemoryType;
  limit?: number;
}

export interface ListMemoriesOutput {
  project_id: string;
  memories: MemorySummary[];
}
