import { ListSkillsInput, SkillListOutput, SkillRollbackInput, SkillSummary } from '../types/skill';

export interface SkillApiClient {
  list(input: ListSkillsInput): Promise<SkillListOutput>;
  rollback?(input: SkillRollbackInput): Promise<SkillSummary>;
}

interface InjectedBridgeWindow {
  __TAURI_INTERNALS__?: {
    invoke?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
  };
  __TAURI_INVOKE__?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
}

const MAX_SKILLS_PER_PAGE = 50;

export class DesktopSkillApiClient implements SkillApiClient {
  async list(input: ListSkillsInput): Promise<SkillListOutput> {
    const normalized = normalizeListInput(input);
    const invoker = bridgeInvoker();
    if (typeof invoker === 'function') {
      return await invoker<SkillListOutput>('list_skills', { input: normalized });
    }

    return {
      project_id: normalized.project_id,
      scope: normalized.scope ?? 'project',
      skills: [],
      total: 0,
      limit: normalized.limit ?? MAX_SKILLS_PER_PAGE,
      offset: normalized.offset ?? 0,
      available: false,
    };
  }

  async rollback(input: SkillRollbackInput): Promise<SkillSummary> {
    const normalized = normalizeRollbackInput(input);
    const invoker = bridgeInvoker();
    if (typeof invoker !== 'function') {
      throw new Error('Serviço de rollback de skills indisponível.');
    }
    return await invoker<SkillSummary>('rollback_skill', { input: normalized });
  }
}

function bridgeInvoker() {
  if (typeof window === 'undefined') return undefined;
  const bridgeWin = window as unknown as InjectedBridgeWindow;
  return bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
}

function normalizeListInput(input: ListSkillsInput): ListSkillsInput {
  if (!isBoundedIdentifier(input.project_id)) {
    throw new Error('Identidade do projeto inválida.');
  }
  if (input.scope && input.scope !== 'project' && input.scope !== 'global') {
    throw new Error('Escopo de skill inválido.');
  }
  return {
    project_id: input.project_id.trim(),
    scope: input.scope,
    limit: Math.min(Math.max(input.limit ?? MAX_SKILLS_PER_PAGE, 1), MAX_SKILLS_PER_PAGE),
    offset: Math.max(input.offset ?? 0, 0),
  };
}

function normalizeRollbackInput(input: SkillRollbackInput): SkillRollbackInput {
  for (const value of [input.project_id, input.skill_id, input.actor_id, input.trace_id]) {
    if (!isBoundedIdentifier(value)) {
      throw new Error('Contexto de rollback de skill inválido.');
    }
  }
  if (input.capability !== 'skill.rollback' || input.confirmed !== true || !Number.isSafeInteger(input.expected_revision)) {
    throw new Error('Contexto de rollback de skill inválido.');
  }
  return input;
}

function isBoundedIdentifier(value: string): boolean {
  return value.trim().length > 0 && value.length <= 128 && !hasControlCharacters(value);
}

function hasControlCharacters(value: string): boolean {
  return Array.from(value).some((character) => {
    const code = character.codePointAt(0) ?? 0;
    return code <= 0x1f || code === 0x7f;
  });
}

export const defaultSkillApi = new DesktopSkillApiClient();
