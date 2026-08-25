import { useState } from 'react';
import { WorkflowEditorModel, type WorkflowApi } from '../contracts/workflow-editor';

type Props = { model: WorkflowEditorModel; api: WorkflowApi; expectedVersion: number; onSaved?: (version: number) => void };

export function WorkflowEditor({ model, api, expectedVersion, onSaved }: Props) {
  const [label, setLabel] = useState('');
  const [kind, setKind] = useState('agent');
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState('');
  const addNode = () => {
    const id = `${kind}-${model.nodes.length + 1}`;
    if (!model.addNode({ id, kind, label })) { setError('Não foi possível adicionar o nó.'); return; }
    setLabel(''); setError(null); setStatus('Nó adicionado ao draft.');
  };
  const save = async () => {
    try { const version = await model.submit(api, expectedVersion); setStatus(`Versão ${version} salva.`); setError(null); onSaved?.(version); }
    catch (reason) { setError(reason instanceof Error ? reason.message : 'Falha ao salvar workflow.'); }
  };
  return <section aria-label="Editor de workflow">
    <h2>Workflow editor</h2>
    <div role="group" aria-label="Adicionar nó">
      <label>Tipo <input value={kind} onChange={(event) => setKind(event.target.value)} /></label>
      <label>Label <input value={label} onChange={(event) => setLabel(event.target.value)} /></label>
      <button type="button" onClick={addNode}>Adicionar nó</button>
    </div>
    <ul aria-label="Nós do workflow">{model.nodes.map((node) => <li key={node.id}><strong>{node.kind}</strong>: {node.label}</li>)}</ul>
    <p aria-label="Arestas">Arestas: {model.edges.length}</p>
    {error && <p role="alert">{error}</p>}
    {status && <p role="status">{status}</p>}
    <button type="button" onClick={save}>Validar e salvar versão</button>
  </section>;
}
