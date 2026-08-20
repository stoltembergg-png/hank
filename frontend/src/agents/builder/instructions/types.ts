import { AgentInstructionApiClient, AgentInstructionSnapshot } from '@/api/agent-instructions';

export interface InstructionsPageProps {
  projectId: string;
  agentId: string;
  onBack: () => void;
  onSaved?: (snapshot: AgentInstructionSnapshot) => void;
  apiClient?: AgentInstructionApiClient;
}

export interface InstructionsFormState {
  content: string;
  initialContent: string;
  maxTotalBytes: number;
  expectedVersion: string;
  provenance: AgentInstructionSnapshot['provenance'];
  isSubmitting: boolean;
  error: string | null;
}