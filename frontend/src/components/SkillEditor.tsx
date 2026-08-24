import React, { useCallback, useEffect, useRef, useState } from 'react';
import { defaultSkillEditorApi, SkillEditorApiClient } from '../api/skillEditor';
import {
  SkillDraftResult,
  SkillEditorDocument,
  SkillEditorFile,
  SkillValidationResult,
} from '../types/skillEditor';

export interface SkillEditorProps {
  projectId: string;
  skillId: string;
  actorId?: string;
  apiClient?: SkillEditorApiClient;
}

export const SkillEditor: React.FC<SkillEditorProps> = ({
  projectId,
  skillId,
  actorId = 'desktop-operator',
  apiClient = defaultSkillEditorApi,
}) => {
  const [loaded, setLoaded] = useState<SkillEditorDocument | null>(null);
  const [manifestJson, setManifestJson] = useState('');
  const [markdown, setMarkdown] = useState('');
  const [files, setFiles] = useState<SkillEditorFile[]>([]);
  const [validation, setValidation] = useState<SkillValidationResult | null>(null);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isBusy, setIsBusy] = useState(false);
  const requestSequence = useRef(0);

  const loadEditor = useCallback(async () => {
    const requestId = ++requestSequence.current;
    setIsLoading(true);
    setError(null);
    setStatusMessage(null);
    setValidation(null);
    try {
      const result = await apiClient.load({ project_id: projectId, skill_id: skillId });
      if (requestId !== requestSequence.current) return;
      validateLoadedDocument(result, projectId, skillId);
      setLoaded(result);
      setManifestJson(result.manifest_json);
      setMarkdown(result.markdown);
      setFiles(result.files);
    } catch (reason) {
      if (requestId !== requestSequence.current) return;
      setLoaded(null);
      setError(reason instanceof Error ? reason.message : 'Falha ao carregar o editor de skills.');
    } finally {
      if (requestId === requestSequence.current) setIsLoading(false);
    }
  }, [apiClient, projectId, skillId]);

  useEffect(() => {
    void loadEditor();
  }, [loadEditor]);

  const buildDocument = () => `---\n${manifestJson}\n---\n${markdown}`;

  const validateDraft = async (): Promise<SkillValidationResult | null> => {
    if (!loaded || isBusy) return null;
    setIsBusy(true);
    setError(null);
    setStatusMessage(null);
    try {
      const result = await apiClient.validate({
        project_id: projectId,
        skill_id: skillId,
        base_version: loaded.base_version,
        document: buildDocument(),
        files,
      });
      setValidation(result);
      if (result.valid && !result.quarantined) setStatusMessage('Rascunho validado; a versão ativa não foi alterada.');
      return result;
    } catch (reason) {
      setValidation(null);
      setError(reason instanceof Error ? reason.message : 'Falha ao validar o rascunho.');
      return null;
    } finally {
      setIsBusy(false);
    }
  };

  const saveDraft = async () => {
    if (!loaded || isBusy) return;
    const result = validation?.valid && !validation.quarantined ? validation : await validateDraft();
    if (!result || !result.valid || result.quarantined) return;
    if (!window.confirm('Salvar este conteúdo como nova versão de rascunho? A versão ativa permanecerá inalterada.')) return;

    setIsBusy(true);
    setError(null);
    setStatusMessage(null);
    try {
      const saved = await apiClient.saveDraft({
        project_id: projectId,
        skill_id: skillId,
        actor_id: actorId,
        trace_id: loaded.trace_id,
        expected_revision: loaded.revision,
        base_version: loaded.base_version,
        document: buildDocument(),
        files,
        budget: loaded.budget,
        policy: { allow: true, max_document_bytes: 64 * 1024 },
        capability: 'skill.edit',
        confirmed: true,
      });
      applyResult(saved);
      setStatusMessage(`Rascunho ${saved.version} salvo; versão ativa preservada.`);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Falha ao salvar o rascunho.');
    } finally {
      setIsBusy(false);
    }
  };

  const discardDraft = async () => {
    if (!loaded || isBusy) return;
    if (!window.confirm('Descartar a edição local e arquivar o rascunho persistido, se houver?')) return;
    setIsBusy(true);
    setError(null);
    setStatusMessage(null);
    try {
      const discarded = await apiClient.discardDraft({
        project_id: projectId,
        skill_id: skillId,
        actor_id: actorId,
        trace_id: loaded.trace_id,
        expected_revision: loaded.revision,
        version: incrementPatchVersion(loaded.base_version),
        capability: 'skill.discard',
        confirmed: true,
      });
      applyResult(discarded);
      setManifestJson(loaded.manifest_json);
      setMarkdown(loaded.markdown);
      setFiles(loaded.files);
      setValidation(null);
      setStatusMessage('Edição descartada sem alterar a versão ativa.');
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Falha ao descartar a edição.');
    } finally {
      setIsBusy(false);
    }
  };

  const applyResult = (result: SkillDraftResult) => {
    if (result.project_id !== projectId || result.skill_id !== skillId) {
      throw new Error('Resposta do editor fora do projeto selecionado.');
    }
    setLoaded((current) => current ? { ...current, revision: result.revision } : current);
  };

  return (
    <section className="skill-editor" aria-label="Editor de skill">
      <header>
        <h3>Editor de skill</h3>
        <p><code>{skillId}</code> · projeto <code>{projectId}</code></p>
      </header>
      {isLoading && <p role="status" aria-busy="true">Carregando editor...</p>}
      {!isLoading && error && <p role="alert">{error}</p>}
      {!isLoading && loaded && (
        <>
          <label>
            Manifest JSON
            <textarea aria-label="Manifest JSON" value={manifestJson} onChange={(event) => setManifestJson(event.target.value)} rows={8} />
          </label>
          <label>
            Instruções Markdown
            <textarea aria-label="Instruções Markdown" value={markdown} onChange={(event) => setMarkdown(event.target.value)} rows={12} />
          </label>
          <p>Arquivos de referência: {files.length}. Autosave desativado.</p>
          {files.map((file, index) => (
            <label key={file.path}>
              Referência {file.path}
              <textarea
                aria-label={`Referência ${file.path}`}
                value={file.content}
                onChange={(event) => setFiles((current) => current.map((entry, entryIndex) => entryIndex === index ? { ...entry, content: event.target.value } : entry))}
                rows={6}
              />
            </label>
          ))}
          <div className="skill-editor-actions">
            <button type="button" onClick={() => void validateDraft()} disabled={isBusy}>Validar rascunho</button>
            <button type="button" onClick={() => void saveDraft()} disabled={isBusy || !loaded}>Salvar rascunho</button>
            <button type="button" onClick={() => void discardDraft()} disabled={isBusy || !loaded}>Descartar edição local</button>
          </div>
          {validation && (
            <div role="alert" aria-live="polite">
              {validation.quarantined ? 'Rascunho colocado em quarentena.' : validation.valid ? 'Rascunho válido.' : 'Rascunho inválido.'}
              {validation.errors.slice(0, 8).map((message) => <p key={message}>{message}</p>)}
              {validation.diagnostics.slice(0, 8).map((diagnostic) => <p key={`${diagnostic.code}-${diagnostic.line ?? 0}`}>{diagnostic.code} · {diagnostic.severity}{diagnostic.line ? ` · linha ${diagnostic.line}` : ''}</p>)}
            </div>
          )}
          {statusMessage && <p role="status">{statusMessage}</p>}
        </>
      )}
    </section>
  );
};

function validateLoadedDocument(document: SkillEditorDocument, projectId: string, skillId: string) {
  if (document.project_id !== projectId || document.skill_id !== skillId) {
    throw new Error('Resposta do editor fora do projeto selecionado.');
  }
}

function incrementPatchVersion(version: string): string {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(version);
  if (!match) return version;
  return `${match[1]}.${match[2]}.${Number(match[3]) + 1}`;
}
