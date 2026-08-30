import {
  CreateSessionInput,
  CreateSessionOutput,
  ListSessionsInput,
  ListSessionsOutput,
} from '../types/session';

export interface SessionApiClient {
  list(input: ListSessionsInput): Promise<ListSessionsOutput>;
  create(input: CreateSessionInput): Promise<CreateSessionOutput>;
}

export const SESSION_BRIDGE_UNAVAILABLE_CODE = 'SESSION_BRIDGE_UNAVAILABLE' as const;

export class SessionBridgeUnavailableError extends Error {
  readonly code = SESSION_BRIDGE_UNAVAILABLE_CODE;

  constructor() {
    super('Session desktop bridge is unavailable');
    this.name = 'SessionBridgeUnavailableError';
  }
}

interface InjectedBridgeWindow {
  __TAURI_INTERNALS__?: {
    invoke?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
  };
  __TAURI_INVOKE__?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
}

function getSessionInvoker() {
  if (typeof window === 'undefined') return undefined;

  const bridgeWin = window as unknown as InjectedBridgeWindow;
  return bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
}

export class DesktopSessionApiClient implements SessionApiClient {
  async list(input: ListSessionsInput): Promise<ListSessionsOutput> {
    const invoker = getSessionInvoker();
    if (typeof invoker === 'function') {
      return await invoker<ListSessionsOutput>('list_sessions', { input });
    }

    throw new SessionBridgeUnavailableError();
  }

  async create(input: CreateSessionInput): Promise<CreateSessionOutput> {
    const invoker = getSessionInvoker();
    if (typeof invoker === 'function') {
      return await invoker<CreateSessionOutput>('create_session', { input });
    }

    throw new SessionBridgeUnavailableError();
  }
}

export const defaultSessionApi = new DesktopSessionApiClient();
