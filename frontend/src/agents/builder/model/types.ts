import { ModelPolicyApiClient, ModelPolicySnapshot, ModelModality } from '@/api/agent-model-policy';

export interface ModelPolicyPageProps {
  projectId: string;
  agentId: string;
  onBack: () => void;
  onSaved?: (snapshot: ModelPolicySnapshot) => void;
  apiClient?: ModelPolicyApiClient;
}

export interface ModelPolicyFormState {
  provider: string;
  model: string;
  maxTokens: string;
  maxContextTokens: string;
  temperature: string;
  modalities: ModelModality[];
  expectedVersion: string;
  initialData: {
    provider: string;
    model: string;
    maxTokens: string;
    maxContextTokens: string;
    temperature: string;
    modalities: ModelModality[];
  };
  isSubmitting: boolean;
  error: string | null;
}