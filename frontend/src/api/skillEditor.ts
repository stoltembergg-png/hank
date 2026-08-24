import {
  SkillDraftResult,
  SkillEditorDiscardInput,
  SkillEditorFile,
  SkillEditorLoadInput,
  SkillEditorSaveDraftInput,
  SkillEditorValidateInput,
  SkillEditorDocument,
  SkillValidationResult,
} from '../types/skillEditor';

export interface SkillEditorApiClient {
  load(input: SkillEditorLoadInput): Promise<SkillEditorDocument>;
  validate(input: SkillEditorValidateInput): Promise<SkillValidationResult>;
  saveDraft(input: SkillEditorSaveDraftInput): Promise<SkillDraftResult>;
  discardDraft(input: SkillEditorDiscardInput): Promise<SkillDraftResult>;
}

interface InjectedBridgeWindow {
  __TAURI_INTERNALS__?: {
    invoke?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
  };
  __TAURI_INVOKE__?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
}

const MAX_DOCUMENT_BYTES = 64 * 1024;
const MAX_FILES = 32;
const MAX_FILE_BYTES = 16 * 1024;
const MAX_IDENTIFIER_LENGTH = 128;

export class DesktopSkillEditorApiClient implements SkillEditorApiClient {
  async load(input: SkillEditorLoadInput): Promise<SkillEditorDocument> {
    const normalized = normalizeLoadInput(input);
    return invokeRequired<SkillEditorDocument>('get_skill_editor', normalized);
  }

  async validate(input: SkillEditorValidateInput): Promise<SkillValidationResult> {
    const normalized = normalizeValidateInput(input);
    return invokeRequired<SkillValidationResult>('validate_skill_draft', normalized);
  }

  async saveDraft(input: SkillEditorSaveDraftInput): Promise<SkillDraftResult> {
    const normalized = normalizeSaveInput(input);
    return invokeRequired<SkillDraftResult>('save_skill_draft', normalized);
  }

  async discardDraft(input: SkillEditorDiscardInput): Promise<SkillDraftResult> {
    const normalized = normalizeDiscardInput(input);
    return invokeRequired<SkillDraftResult>('discard_skill_draft', normalized);
  }
}

function invokeRequired<T>(command: string, input: unknown): Promise<T> {
  const invoker = bridgeInvoker();
  if (typeof invoker !== 'function') {
    throw new Error('Editor de skills indisponível neste desktop.');
  }
  return invoker<T>(command, { input });
}

function bridgeInvoker() {
  if (typeof window === 'undefined') return undefined;
  const bridgeWin = window as unknown as InjectedBridgeWindow;
  return bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
}

function normalizeLoadInput(input: SkillEditorLoadInput): SkillEditorLoadInput {
  assertIdentifier(input.project_id, 'projeto');
  assertIdentifier(input.skill_id, 'skill');
  if (input.version !== undefined) assertVersion(input.version);
  return {
    project_id: input.project_id.trim(),
    skill_id: input.skill_id.trim(),
    version: input.version?.trim(),
  };
}

function normalizeValidateInput(input: SkillEditorValidateInput): SkillEditorValidateInput {
  assertEditorIdentity(input.project_id, input.skill_id, input.base_version);
  assertDocument(input.document);
  return {
    project_id: input.project_id.trim(),
    skill_id: input.skill_id.trim(),
    base_version: input.base_version.trim(),
    document: input.document,
    files: normalizeFiles(input.files),
  };
}

function normalizeSaveInput(input: SkillEditorSaveDraftInput): SkillEditorSaveDraftInput {
  const normalized = normalizeValidateInput(input);
  assertIdentifier(input.actor_id, 'operador');
  assertIdentifier(input.trace_id, 'trace');
  assertRevision(input.expected_revision);
  if (input.capability !== 'skill.edit' || input.confirmed !== true) {
    throw new Error('Confirmação ou capability de edição inválida.');
  }
  if (input.policy.allow !== true || !Number.isSafeInteger(input.policy.max_document_bytes)
    || input.policy.max_document_bytes < 1 || input.policy.max_document_bytes > MAX_DOCUMENT_BYTES) {
    throw new Error('Policy de edição inválida.');
  }
  return { ...input, ...normalized, actor_id: input.actor_id.trim(), trace_id: input.trace_id.trim() };
}

function normalizeDiscardInput(input: SkillEditorDiscardInput): SkillEditorDiscardInput {
  assertEditorIdentity(input.project_id, input.skill_id, input.version);
  assertIdentifier(input.actor_id, 'operador');
  assertIdentifier(input.trace_id, 'trace');
  assertRevision(input.expected_revision);
  if (input.capability !== 'skill.discard' || input.confirmed !== true) {
    throw new Error('Confirmação ou capability de descarte inválida.');
  }
  return { ...input, project_id: input.project_id.trim(), skill_id: input.skill_id.trim(), version: input.version.trim(), actor_id: input.actor_id.trim(), trace_id: input.trace_id.trim() };
}

function normalizeFiles(files: SkillEditorFile[] | undefined): SkillEditorFile[] {
  if (!files) return [];
  if (files.length > MAX_FILES) throw new Error('Quantidade de arquivos da skill excede o limite.');
  return files.map((file) => {
    if (!file.path || file.path.length > MAX_IDENTIFIER_LENGTH || file.path.includes('..') || file.path.startsWith('/')) {
      throw new Error('Caminho de referência da skill inválido.');
    }
    if (file.content.length > MAX_FILE_BYTES || hasControlCharacters(file.path)) {
      throw new Error('Arquivo de referência da skill excede o limite.');
    }
    return { path: file.path.trim(), role: file.role.slice(0, 64), content: file.content };
  });
}

function assertDocument(document: string) {
  if (new TextEncoder().encode(document).length > MAX_DOCUMENT_BYTES) {
    throw new Error('Documento da skill excede o limite de tamanho.');
  }
}

function assertEditorIdentity(projectId: string, skillId: string, version: string) {
  assertIdentifier(projectId, 'projeto');
  assertIdentifier(skillId, 'skill');
  assertVersion(version);
}

function assertIdentifier(value: string, label: string) {
  if (typeof value !== 'string' || value.trim().length === 0 || value.length > MAX_IDENTIFIER_LENGTH || hasControlCharacters(value)) {
    throw new Error(`Identidade de ${label} inválida.`);
  }
}

function assertVersion(value: string) {
  if (typeof value !== 'string' || !/^\d+\.\d+\.\d+$/.test(value.trim())) {
    throw new Error('Versão da skill inválida.');
  }
}

function assertRevision(value: number) {
  if (!Number.isSafeInteger(value) || value < 1) throw new Error('Revisão da skill inválida.');
}

function hasControlCharacters(value: string): boolean {
  return Array.from(value).some((character) => {
    const code = character.codePointAt(0) ?? 0;
    return code <= 0x1f || code === 0x7f;
  });
}

export const defaultSkillEditorApi = new DesktopSkillEditorApiClient();
