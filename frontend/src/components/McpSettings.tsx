import type { RevokeScope } from '../api/mcp-settings';

export type McpServer = { id: string; name: string; endpoint: string; capabilities: string[]; trust: 'authorized' | 'pending' | 'disabled' };
export type McpTool = { name: string; state: 'Pending' | 'Disabled' };

export function McpSettings({ servers, tools, onRevoke }: { servers: McpServer[]; tools: McpTool[]; onRevoke: (scope: RevokeScope) => void }) {
  return (
    <section aria-labelledby="mcp-settings-title">
      <h2 id="mcp-settings-title">MCP</h2>
      <p>Servidores autorizados e tools staged permanecem sob revisão explícita.</p>
      <div aria-label="Servidores MCP">
        {servers.map((server) => (
          <article key={server.id}>
            <h3>{server.name}</h3>
            <p>{server.endpoint}</p>
            <p>Trust: {server.trust}</p>
            <p>Capabilities: {server.capabilities.join(', ') || 'nenhuma'}</p>
            <button type="button" onClick={() => onRevoke({ server_id: server.id, project_id: 'local' })}>Revogar MCP</button>
          </article>
        ))}
      </div>
      <div aria-label="Tools MCP staged">
        <h3>Tools staged</h3>
        {tools.map((tool) => (
          <article key={tool.name}>
            <span>{tool.name}</span>
            <span>{tool.state === 'Pending' ? 'Pendente — ativação manual necessária' : 'Desabilitada'}</span>
          </article>
        ))}
      </div>
    </section>
  );
}
