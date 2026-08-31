import { useMemo, useState } from 'react';
import {
  WorkflowEditorModel,
  type WorkflowApi,
  type WorkflowNode,
} from '../contracts/workflow-editor';
import './WorkflowSurface.css';

export interface WorkflowSurfaceProps {
  projectId: string;
  api?: WorkflowApi;
  expectedVersion?: number;
}

type NodeKind = 'agent' | 'condition' | 'approval' | 'tool';

type NodeDefinition = {
  kind: NodeKind;
  label: string;
  description: string;
  icon: string;
};

const NODE_DEFINITIONS: readonly NodeDefinition[] = [
  { kind: 'agent', label: 'Agent', description: 'Executa uma etapa com um agente', icon: '✦' },
  { kind: 'condition', label: 'Condition', description: 'Avalia uma condição do fluxo', icon: '?' },
  { kind: 'approval', label: 'Approval', description: 'Aguarda uma aprovação humana', icon: '✓' },
  { kind: 'tool', label: 'Tool', description: 'Executa uma ferramenta autorizada', icon: '▣' },
];

const NODE_DEFINITION_BY_KIND: ReadonlyMap<string, NodeDefinition> = new Map(
  NODE_DEFINITIONS.map((definition) => [definition.kind, definition]),
);

export function WorkflowSurface({
  projectId,
  api,
  expectedVersion = 0,
}: WorkflowSurfaceProps) {
  const model = useMemo(
    () => new WorkflowEditorModel(projectId, 'workflow-draft', 12, 24, 256),
    [projectId],
  );
  const [draftRevision, setDraftRevision] = useState(0);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [status, setStatus] = useState('');
  const [error, setError] = useState<string | null>(null);

  const addNode = (definition: NodeDefinition) => {
    const nextNumber = model.nodes.filter((node) => node.kind === definition.kind).length + 1;
    const node: WorkflowNode = {
      id: `${definition.kind}-${nextNumber}`,
      kind: definition.kind,
      label: `${definition.label} ${nextNumber}`,
    };

    if (!model.addNode(node)) {
      setError('Não foi possível adicionar o nó ao rascunho.');
      return;
    }

    setSelectedNodeId(node.id);
    setDraftRevision((revision) => revision + 1);
    setError(null);
    setStatus(`${definition.label} adicionado ao rascunho.`);
  };

  const save = async () => {
    if (!api) return;

    try {
      const version = await model.submit(api, expectedVersion);
      setStatus(`Workflow salvo na versão ${version}.`);
      setError(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Falha ao salvar workflow.');
    }
  };

  const selectedNode = model.nodes.find((node) => node.id === selectedNodeId);

  return (
    <section
      className="workflow-surface"
      aria-label="Workflows do projeto"
      data-project-id={projectId}
      data-draft-revision={draftRevision}
    >
      <header className="workflow-surface-header">
        <div>
          <p className="workflow-surface-eyebrow">Workflow studio</p>
          <h2>Workflow studio</h2>
          <p>Modele um fluxo visual com etapas bounded e rastreáveis.</p>
        </div>
        <span className="workflow-surface-badge">Rascunho local</span>
      </header>

      <div className="workflow-surface-notice" role="status">
        {api
          ? 'A persistência está disponível através da ponte do workflow.'
          : 'A persistência de workflows ainda não está disponível no desktop.'}
      </div>

      <div className="workflow-surface-layout">
        <aside className="workflow-node-palette" aria-label="Biblioteca de nós">
          <div className="workflow-panel-heading">
            <div>
              <p className="workflow-panel-eyebrow">Building blocks</p>
              <h3>Biblioteca</h3>
            </div>
            <span>{NODE_DEFINITIONS.length} tipos</span>
          </div>
          <div className="workflow-node-options">
            {NODE_DEFINITIONS.map((definition) => (
              <button
                key={definition.kind}
                type="button"
                className="workflow-node-option"
                onClick={() => addNode(definition)}
                aria-label={`Adicionar nó ${definition.label}`}
              >
                <span className={`workflow-node-option-icon workflow-node-option-icon--${definition.kind}`} aria-hidden="true">
                  {definition.icon}
                </span>
                <span className="workflow-node-option-copy">
                  <strong>{definition.label}</strong>
                  <small>{definition.description}</small>
                </span>
                <span className="workflow-node-option-plus" aria-hidden="true">＋</span>
              </button>
            ))}
          </div>
        </aside>

        <main className="workflow-canvas" aria-label="Canvas do workflow">
          <div className="workflow-canvas-toolbar">
            <div>
              <p className="workflow-panel-eyebrow">Canvas</p>
              <h3>New workflow</h3>
            </div>
            <span className="workflow-canvas-state">
              <span aria-hidden="true" />
              Draft
            </span>
          </div>
          <div className="workflow-canvas-grid">
            {model.nodes.length === 0 ? (
              <div className="workflow-canvas-empty" role="status">
                <span className="workflow-canvas-empty-icon" aria-hidden="true">◇</span>
                <strong>Comece pelo primeiro nó</strong>
                <span>Escolha uma etapa na biblioteca para montar o fluxo.</span>
              </div>
            ) : (
              <ul className="workflow-node-list" aria-label="Nós do workflow">
                {model.nodes.map((node) => {
                  const definition = NODE_DEFINITION_BY_KIND.get(node.kind);
                  const isSelected = selectedNodeId === node.id;

                  return (
                    <li key={node.id} className="workflow-node-list-item" aria-label={node.label}>
                      <button
                        type="button"
                        className={`workflow-node-card${isSelected ? ' is-selected' : ''}`}
                        onClick={() => setSelectedNodeId(node.id)}
                        aria-pressed={isSelected}
                      >
                        <span className={`workflow-node-card-icon workflow-node-card-icon--${node.kind}`} aria-hidden="true">
                          {definition?.icon ?? '◇'}
                        </span>
                        <span className="workflow-node-card-copy">
                          <strong>{node.label}</strong>
                          <small>{definition?.label ?? node.kind}</small>
                        </span>
                        <span className="workflow-node-card-status" aria-label="Nó no draft">●</span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
          <div className="workflow-canvas-footer">
            <span>{model.nodes.length} nós</span>
            <span aria-hidden="true">·</span>
            <span>{model.edges.length} arestas</span>
            <span className="workflow-canvas-footer-hint">Edges serão adicionadas na próxima etapa.</span>
          </div>
        </main>

        <aside className="workflow-inspector" aria-label="Inspetor do workflow">
          <div className="workflow-panel-heading">
            <div>
              <p className="workflow-panel-eyebrow">Properties</p>
              <h3>Inspetor</h3>
            </div>
            <span>Draft</span>
          </div>
          {selectedNode ? (
            <dl className="workflow-inspector-details">
              <div>
                <dt>Nome</dt>
                <dd>{selectedNode.label}</dd>
              </div>
              <div>
                <dt>Tipo</dt>
                <dd>{NODE_DEFINITION_BY_KIND.get(selectedNode.kind)?.label ?? selectedNode.kind}</dd>
              </div>
              <div>
                <dt>Identificador</dt>
                <dd className="workflow-inspector-id">{selectedNode.id}</dd>
              </div>
            </dl>
          ) : (
            <div className="workflow-inspector-empty">
              <span aria-hidden="true">⌁</span>
              <p>Selecione um nó para ver os detalhes.</p>
            </div>
          )}
        </aside>
      </div>

      <footer className="workflow-surface-footer">
        <div>
          <strong>Draft protegido por limites do projeto</strong>
          <span>Máximo de 12 nós e 24 arestas nesta superfície.</span>
        </div>
        <button
          type="button"
          className="workflow-save-button"
          onClick={save}
          disabled={!api || model.nodes.length === 0}
          title={api ? 'Salvar workflow' : 'A persistência ainda não está disponível no desktop'}
        >
          Salvar workflow
        </button>
      </footer>

      {error && <p className="workflow-surface-error" role="alert">{error}</p>}
      {status && <p className="workflow-surface-status" role="status">{status}</p>}
    </section>
  );
}
