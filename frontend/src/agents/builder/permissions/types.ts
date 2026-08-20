import {
  PermissionAction,
  PermissionEffect,
  PermissionResource,
  PermissionScope,
  ToolPermissionApiClient,
  ToolPermissionPolicy,
  ToolPermissionPolicySnapshot,
} from '@/api/agent-tool-permissions';

export interface PermissionsPageProps {
  projectId: string;
  agentId: string;
  onBack: () => void;
  onSaved?: (snapshot: ToolPermissionPolicySnapshot) => void;
  apiClient?: ToolPermissionApiClient;
}

export interface PermissionDraft {
  resource: PermissionResource;
  action: PermissionAction;
  effect: PermissionEffect;
  scope: PermissionScope;
  scopeId: string;
  expiresAt: string;
}

export interface PermissionsFormState {
  policy: ToolPermissionPolicy | null;
  initialPolicy: ToolPermissionPolicy | null;
  expectedVersion: string;
  draft: PermissionDraft;
  isSubmitting: boolean;
  error: string | null;
}