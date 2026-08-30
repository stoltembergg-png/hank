export type SessionStatus = 'created' | 'active' | 'closing' | 'closed' | 'failed';

export interface SessionSummary {
  id: string;
  project_id: string;
  agent_id: string;
  status: SessionStatus;
  title?: string | null;
  message_count: number;
  token_count: number;
  created_at: string;
  updated_at: string;
  closed_at?: string | null;
}

export interface ListSessionsInput {
  project_id: string;
  agent_id: string;
  limit?: number;
  offset?: number;
  correlation_id?: string | null;
}

export interface ListSessionsOutput {
  sessions: SessionSummary[];
  total: number;
  limit: number;
  offset: number;
  correlation_id?: string | null;
}

export interface CreateSessionInput {
  project_id: string;
  agent_id: string;
  title?: string | null;
  correlation_id: string;
}

export interface CreateSessionOutput {
  session: SessionSummary;
  correlation_id: string;
}
