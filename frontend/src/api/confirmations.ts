import type { ConfirmationRequest } from '@/contracts/confirmation';

export type ApprovalGrant = {
  grant_id: string;
  request_id: string;
  actor_id: string;
  request_fingerprint: string;
  scope_fingerprint: string;
  expires_at_ms: number;
};

export interface ApproveConfirmationInput {
  request_id: string;
  actor_id: string;
  now_ms: number;
}

export interface ConfirmationApiClient {
  submit(request: ConfirmationRequest): Promise<ConfirmationRequest>;
  approve(input: ApproveConfirmationInput): Promise<ApprovalGrant>;
  revoke(request: ConfirmationRequest): Promise<void>;
}

type BridgeInvoker = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

interface InjectedBridgeWindow {
  __TAURI_INTERNALS__?: { invoke?: BridgeInvoker };
  __TAURI_INVOKE__?: BridgeInvoker;
}

function bridgeInvoker(): BridgeInvoker | undefined {
  if (typeof window === 'undefined') return undefined;
  const bridgeWin = window as unknown as InjectedBridgeWindow;
  return bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
}

/**
 * Typed client for the confirmation lifecycle commands. Every call
 * transports only bounded artifacts and inputs; raw schemas and arguments
 * never cross this client.
 */
export class DesktopConfirmationApiClient implements ConfirmationApiClient {
  private readonly configuredInvoker?: BridgeInvoker;

  public constructor(invoker?: BridgeInvoker) {
    this.configuredInvoker = invoker;
  }

  public async submit(request: ConfirmationRequest): Promise<ConfirmationRequest> {
    return await this.invoke<ConfirmationRequest>('submit_confirmation_request', { request });
  }

  public async approve(input: ApproveConfirmationInput): Promise<ApprovalGrant> {
    return await this.invoke<ApprovalGrant>('approve_confirmation_request', { input });
  }

  public async revoke(request: ConfirmationRequest): Promise<void> {
    await this.invoke<void>('revoke_confirmation_request', { request });
  }

  private async invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    const invoker = this.configuredInvoker ?? bridgeInvoker();
    if (typeof invoker !== 'function') {
      throw new Error('No confirmation service available');
    }
    return await invoker<T>(cmd, args);
  }
}
