import { RunViewerModel } from '../contracts/workflow-run-viewer';

type Props = { viewer: RunViewerModel };

export function WorkflowRunViewer({ viewer }: Props) {
  const snapshot = viewer.snapshot;
  if (!snapshot) return <section aria-label="Workflow run viewer"><p role="status">Nenhum run selecionado.</p></section>;
  return <section aria-label="Workflow run viewer">
    <h2>Run {snapshot.run_id}</h2>
    <p role="status" aria-label="Estado do run">Estado: {viewer.displayState}</p>
    <ol aria-label="Estado dos nós">{snapshot.nodes.map((node) => <li key={node.node_id}>{node.node_id}: {node.state}</li>)}</ol>
    <ol aria-label="Timeline">{viewer.timeline.map((event) => <li key={event.sequence}>{event.kind}: {event.message}</li>)}</ol>
  </section>;
}
