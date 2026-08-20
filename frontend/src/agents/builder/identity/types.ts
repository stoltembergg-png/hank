/**
 * Tipos estendidos para a página de identidade do Agent.
 * Conforme PR-050 e fronteiras de isolamento.
 */

import { AgentSummary } from '@/types/agent';
import { AgentApiClient } from '@/api/agents';

export interface AgentIdentityFormData {
  name: string;
  description: string;
}

export interface AgentIdentityPageProps {
  projectId: string;
  agentId: string;
  onBack: () => void;
  onSaved?: (agent: AgentSummary) => void;
  apiClient?: AgentApiClient;
}

export interface AgentIdentityFormState {
  name: string;
  description: string;
  isSubmitting: boolean;
  error: string | null;
  expectedVersion: string;
  initialData: AgentIdentityFormData;
}