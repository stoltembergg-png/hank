/**
 * Contratos de tipo para gerenciamento de Agents no frontend.
 * Conforme PR-048, PR-049 e fronteiras de isolamento.
 */

export type AgentStatus = 'active' | 'inactive' | 'suspended';

export interface CommunicationStyle {
  formal: 'formal';
  casual: 'casual';
  technical: 'technical';
  concise: 'concise';
  verbose: 'verbose';
}

export interface Personality {
  name: string;
  description?: string | null;
  traits: string[];
  communication_style: keyof CommunicationStyle;
}

export interface AgentSummary {
  id: string;
  project_id: string;
  name: string;
  description?: string | null;
  status: AgentStatus;
  personality: Personality;
  created_at: string;
  updated_at: string;
}

export interface ListAgentsInput {
  project_id: string;
  limit?: number;
  offset?: number;
}

export interface ListAgentsOutput {
  agents: AgentSummary[];
  total: number;
  limit: number;
  offset: number;
}

export interface CreateAgentInput {
  project_id: string;
  name: string;
  description?: string | null;
  policy?: Record<string, unknown>;
  correlation_id?: string | null;
}

export interface CreateAgentOutput {
  agent: AgentSummary;
  event_id?: string | null;
  correlation_id?: string | null;
}

export interface UpdateAgentInput {
  project_id: string;
  agent_id: string;
  name?: string;
  description?: string | null;
  status?: AgentStatus;
  personality?: Personality;
  policy?: Record<string, unknown>;
  expected_version: string;
  correlation_id?: string | null;
}

export interface UpdateAgentOutput {
  agent: AgentSummary;
  event_id?: string | null;
  correlation_id?: string | null;
}

export interface ArchiveAgentInput {
  project_id: string;
  agent_id: string;
  expected_version: string;
  confirmation: string;
  correlation_id?: string | null;
}

export interface ArchiveAgentOutput {
  agent: AgentSummary;
  event_id?: string | null;
  correlation_id?: string | null;
}