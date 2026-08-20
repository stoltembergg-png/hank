import {
  ModelCapabilities,
  ModelModality,
  ModelPolicy,
} from './agent-model-policy';

export type ModelSelectorOptionState = 'available' | 'disabled' | 'expired' | 'unavailable';
export type ModelDiscoverySource = 'provider' | 'cache';

export interface ModelSelectorOption {
  provider_id: string;
  model_id: string;
  display_name: string;
  capabilities: ModelCapabilities;
  state: ModelSelectorOptionState;
  reason?: string;
  source: ModelDiscoverySource;
}

export interface ModelSelectorSnapshot {
  project_id: string;
  agent_id: string;
  policy: ModelPolicy;
  options: ModelSelectorOption[];
  updated_at: string;
}

export interface UpdateModelSelectionInput {
  project_id: string;
  agent_id: string;
  provider_id: string;
  model_id: string;
  expected_version: string;
}

export interface ModelSelectorApiClient {
  get(projectId: string, agentId: string): Promise<ModelSelectorSnapshot>;
  update(input: UpdateModelSelectionInput): Promise<ModelSelectorSnapshot>;
}

function bridgeInvoker(): (<T>(command: string, args?: Record<string, unknown>) => Promise<T>) | undefined {
  if (typeof window === 'undefined') return undefined;
  const bridge = window as unknown as {
    __TAURI_INTERNALS__?: { invoke?: <T>(command: string, args?: Record<string, unknown>) => Promise<T> };
    __TAURI_INVOKE__?: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
  };
  return bridge.__TAURI_INTERNALS__?.invoke ?? bridge.__TAURI_INVOKE__;
}

export class DesktopModelSelectorApiClient implements ModelSelectorApiClient {
  async get(projectId: string, agentId: string): Promise<ModelSelectorSnapshot> {
    const invoke = bridgeInvoker();
    if (!invoke) throw new Error('Model discovery service unavailable');
    return invoke<ModelSelectorSnapshot>('get_model_selector_snapshot', { projectId, agentId });
  }

  async update(input: UpdateModelSelectionInput): Promise<ModelSelectorSnapshot> {
    const invoke = bridgeInvoker();
    if (!invoke) throw new Error('Model policy service unavailable');
    return invoke<ModelSelectorSnapshot>('update_model_selection', { input });
  }
}

export const defaultModelSelectorApi = new DesktopModelSelectorApiClient();

export function isModelOptionCompatible(
  option: ModelSelectorOption,
  modalities: ModelModality[],
): boolean {
  if (option.state !== 'available') return false;
  if (!isSafeIdentifier(option.provider_id) || !isSafeIdentifier(option.model_id)) return false;
  return modalities.every((modality) => option.capabilities[modality] === 'supported');
}

function isSafeIdentifier(value: string): boolean {
  const hasControlOrWhitespace = [...value].some((character) => {
    const code = character.charCodeAt(0);
    return code <= 0x1f || code === 0x7f || /\s/.test(character);
  });
  return value.length > 0
    && value.length <= 200
    && !hasControlOrWhitespace
    && !value.includes('://');
}

export function optionKey(option: Pick<ModelSelectorOption, 'provider_id' | 'model_id'>): string {
  return `${option.provider_id}:${option.model_id}`;
}
