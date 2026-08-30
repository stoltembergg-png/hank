import React from 'react';
import { SessionSummary } from '../types/session';
import './SessionWorkbench.css';

export interface SessionWorkbenchProps {
  session: SessionSummary;
  agentName: string;
  onBack?: () => void;
}

const formatMessageCount = (count: number): string =>
  count === 1 ? '1 mensagem registrada' : `${count} mensagens registradas`;

export const SessionWorkbench: React.FC<SessionWorkbenchProps> = ({
  session,
  agentName,
  onBack,
}) => {
  const title = session.title?.trim() || 'Conversa sem título';

  return (
    <section className="session-workbench" aria-label={`Conversa ${title}`}>
      <header className="session-workbench-header">
        <div>
          <p className="session-workbench-eyebrow">Conversa com {agentName}</p>
          <h3>{title}</h3>
        </div>
        {onBack && (
          <button type="button" className="session-workbench-back" onClick={onBack}>
            Voltar para conversas
          </button>
        )}
      </header>

      <dl className="session-workbench-details">
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
          <dd>{formatMessageCount(session.message_count)}</dd>
        </div>
        <div>
          <dt>ID da sessão</dt>
          <dd className="session-workbench-id">{session.id}</dd>
        </div>
      </dl>

      <div className="session-workbench-notice" role="status">
        <strong>Execução ainda não conectada</strong>
        <p>
          {session.message_count === 0
            ? 'Nenhuma mensagem foi registrada nesta sessão.'
            : `Esta sessão possui ${formatMessageCount(session.message_count)}.`}
        </p>
        <p>Envio de mensagens ainda não está integrado ao desktop.</p>
      </div>

      <fieldset className="session-workbench-composer" disabled aria-label="Área de composição da conversa">
        <legend>Nova mensagem</legend>
        <label htmlFor={`session-message-${session.id}`}>Mensagem</label>
        <textarea
          id={`session-message-${session.id}`}
          rows={4}
          placeholder="O envio estará disponível quando o bridge de execução for integrado."
        />
        <button type="button">Enviar mensagem</button>
      </fieldset>
    </section>
  );
};
