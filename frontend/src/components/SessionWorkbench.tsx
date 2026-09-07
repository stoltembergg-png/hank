import React from 'react';
import { SessionSummary } from '../types/session';
import { ChatPage } from '../chat/ChatPage';
import type { ChatTransport } from '../chat/ChatPage';
import { defaultChatTransport } from '../api/chat';
import './SessionWorkbench.css';

export interface SessionWorkbenchProps {
  session: SessionSummary;
  agentName: string;
  onBack?: () => void;
  transport?: ChatTransport;
}

export const SessionWorkbench: React.FC<SessionWorkbenchProps> = ({
  session,
  agentName,
  onBack,
  transport = defaultChatTransport,
}) => {
  const title = session.title?.trim() || 'Conversa sem título';

  return (
    <section className="session-workbench" aria-label={`Conversa ${title}`}>
      <header className="session-workbench-header">
        <div className="session-workbench-agent">
          <span className="session-workbench-agent-avatar" aria-hidden="true">✦</span>
        <div>
          <p className="session-workbench-eyebrow">Conversa com {agentName}</p>
          <h3>{title}</h3>
          </div>
        </div>
        {onBack && (
          <button type="button" className="session-workbench-back" onClick={onBack}>
            Voltar para conversas
          </button>
        )}
      </header>

      <dl className="session-workbench-details" role="group" aria-label="Resumo da sessão">
        <div>
          <dt>Status</dt>
          <dd>
            <span className={`session-status session-status--${session.status}`}>
              {session.status}
            </span>
          </dd>
        </div>
        <div>
          <dt>Mensagens</dt>
          <dd>{session.message_count}</dd>
        </div>
        <div>
          <dt>ID da sessão</dt>
          <dd className="session-workbench-id">{session.id}</dd>
        </div>
      </dl>

      <div className="session-workbench-conversation" role="region" aria-label="Área da conversa">
        <ChatPage
          session={{
            caller: { caller_id: 'desktop-webview', class: 'desktop' },
            project_id: session.project_id,
            agent_id: session.agent_id,
            session_id: session.id,
            generation: 1,
          }}
          transport={transport}
        />
      </div>
    </section>
  );
};
