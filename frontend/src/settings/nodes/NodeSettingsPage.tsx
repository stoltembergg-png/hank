import { useState } from 'react';
import { NodeList } from './NodeList';
import { NodeDetail } from './NodeDetail';
import {
  NodeManagementApiClient,
  defaultNodeManagementApi,
} from '../../api/node-management';

interface NodeSettingsPageProps {
  projectId: string;
  apiClient?: NodeManagementApiClient;
  onBack?: () => void;
}

export function NodeSettingsPage({
  projectId,
  apiClient = defaultNodeManagementApi,
  onBack,
}: NodeSettingsPageProps) {
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  return (
    <section className="node-settings" aria-label="Gerenciamento de nodes remotos">
      <header className="node-settings__header">
        <h1>Nodes remotos autenticados</h1>
        {onBack ? (
          <button type="button" onClick={onBack}>
            Voltar
          </button>
        ) : null}
      </header>
      {error ? (
        <p role="alert" className="node-settings__error">
          {error}
        </p>
      ) : null}
      <div className="node-settings__layout">
        <NodeList
          projectId={projectId}
          apiClient={apiClient}
          onSelect={setSelectedNodeId}
          onError={(message) => setError(message)}
        />
        <NodeDetail
          projectId={projectId}
          nodeId={selectedNodeId}
          apiClient={apiClient}
          onClose={() => setSelectedNodeId(null)}
        />
      </div>
    </section>
  );
}
