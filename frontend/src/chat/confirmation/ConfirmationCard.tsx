import type { ConfirmationApiClient } from '@/api/confirmations';
import type { ConfirmationRequest } from '@/contracts/confirmation';
import './ConfirmationCard.css';

const POLICY_LABELS: Record<ConfirmationRequest['policy'], string> = {
  always_allow: 'Sempre permitir',
  ask_once: 'Perguntar uma vez',
  ask_every_time: 'Perguntar sempre',
  deny: 'Negar',
};

const EFFECT_LABELS: Record<ConfirmationRequest['effect'], string> = {
  read: 'leitura',
  write: 'escrita',
  execute: 'execução',
  network: 'rede',
  credentials: 'credenciais',
  payment: 'pagamento',
  install_package: 'instalação de pacote',
  force_push: 'force push',
};

interface ConfirmationCardProps {
  request: ConfirmationRequest;
  apiClient: ConfirmationApiClient;
  nowMs: number;
}

/**
 * Renders one pending approval with bounded metadata only. Approval and
 * revocation stay bound to the actor and instant presented by the caller;
 * raw schemas and arguments are never displayed.
 */
export function ConfirmationCard({ request, apiClient, nowMs }: ConfirmationCardProps) {
  return (
    <section className="confirmation-card" aria-labelledby="confirmation-card-title">
      <h3 id="confirmation-card-title" className="confirmation-card__title">
        Aprovação necessária
      </h3>
      <p className="confirmation-card__tool">
        {request.tool_name} · versão {request.tool_version}
      </p>
      <p className="confirmation-card__args">Hash dos argumentos: {request.args_hash}</p>
      <p className="confirmation-card__meta">
        Efeito: {EFFECT_LABELS[request.effect]} · Política: {POLICY_LABELS[request.policy]}
      </p>
      <p className="confirmation-card__meta">
        Actor: {request.actor_id} · Expira em: {request.expires_at_ms}
      </p>
      <div className="confirmation-card__actions">
        <button
          type="button"
          className="confirmation-card__approve"
          onClick={() => {
            void apiClient
              .approve({
                request_id: request.request_id,
                actor_id: request.actor_id,
                now_ms: nowMs,
              })
              .catch(() => undefined);
          }}
        >
          Aprovar {request.tool_name}
        </button>
        <button
          type="button"
          className="confirmation-card__revoke"
          onClick={() => {
            void apiClient.revoke(request).catch(() => undefined);
          }}
        >
          Revogar solicitação
        </button>
      </div>
    </section>
  );
}
