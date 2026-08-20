import { AgentApiClient } from '@/api/agents';
import { AgentSummary, Personality } from '@/types/agent';

export interface PersonalityPageProps {
  projectId: string;
  agentId: string;
  onBack: () => void;
  onSaved?: (agent: AgentSummary) => void;
  apiClient?: AgentApiClient;
}

export type PersonalityFormData = Personality;

export interface PersonalityFormState {
  name: string;
  description: string;
  traits: string[];
  communicationStyle: Personality['communication_style'];
  expectedVersion: string;
  initialData: PersonalityFormData;
  isSubmitting: boolean;
  error: string | null;
}