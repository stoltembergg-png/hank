import type {
  WorkflowApi,
  WorkflowCommand,
  WorkflowSnapshot,
  WorkflowValidation,
} from '../contracts/workflow-editor';

type BridgeInvoker = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

interface BridgeWindow {
  __TAURI_INTERNALS__?: { invoke?: BridgeInvoker };
  __TAURI_INVOKE__?: BridgeInvoker;
}

function bridgeInvoker(): BridgeInvoker | undefined {
  if (typeof window === 'undefined') return undefined;
  const bridge = window as unknown as BridgeWindow;
  return bridge.__TAURI_INTERNALS__?.invoke ?? bridge.__TAURI_INVOKE__;
}

export class WorkflowBridgeUnavailableError extends Error {
  readonly code = 'WORKFLOW_BRIDGE_UNAVAILABLE';

  constructor() {
    super('Workflow desktop bridge is unavailable; no local persistence fallback is permitted');
    this.name = 'WorkflowBridgeUnavailableError';
  }
}

export class DesktopWorkflowApiClient implements WorkflowApi {
  private readonly invoke?: BridgeInvoker;

  constructor(invoke?: BridgeInvoker) {
    this.invoke = invoke;
  }

  private getInvoker(): BridgeInvoker {
    const invoke = this.invoke ?? bridgeInvoker();
    if (!invoke) throw new WorkflowBridgeUnavailableError();
    return invoke;
  }

  validate(command: WorkflowCommand): Promise<WorkflowValidation> {
    return this.getInvoker()<WorkflowValidation>('validate_workflow', { input: command });
  }

  save(command: WorkflowCommand): Promise<{ version: number }> {
    return this.getInvoker()<WorkflowSaveOutput>('save_workflow', { input: command });
  }

  load(projectId: string, workflowId: string): Promise<WorkflowSnapshot | null> {
    return this.getInvoker()<WorkflowSnapshot | null>('get_workflow', {
      input: { project_id: projectId, workflow_id: workflowId },
    });
  }
}

type WorkflowSaveOutput = { version: number };

export const defaultWorkflowApi = new DesktopWorkflowApiClient();

export function desktopWorkflowApiOrUndefined(): WorkflowApi | undefined {
  return bridgeInvoker() ? defaultWorkflowApi : undefined;
}
