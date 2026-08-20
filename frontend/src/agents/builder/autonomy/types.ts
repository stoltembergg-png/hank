import { AutonomyApiClient, AutonomyLevel, AutonomyPolicySnapshot } from '@/api/agent-autonomy';

export interface AutonomyPageProps {
  projectId: string;
  agentId: string;
  onBack: () => void;
  onSaved?: (snapshot: AutonomyPolicySnapshot) => void;
  apiClient?: AutonomyApiClient;
}

export interface AutonomyFormState {
  targetLevel: AutonomyLevel;
  maxSteps: string;
  approverId: string;
  reason: string;
  expiresAt: string;
  initialLevel: AutonomyLevel;
  initialMaxSteps: number;
  expectedVersion: string;
  isSubmitting: boolean;
  error: string | null;
}