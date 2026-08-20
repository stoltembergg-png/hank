import { useEffect, useState } from 'react';
import {
  defaultProviderSettingsApi,
  OAuthFlowStatus,
  ProviderAccountStatus,
  ProviderSettingsApiClient,
} from '../../api/provider-settings';
import './ProviderSettingsPage.css';

export type {
  OAuthFlowStatus,
  ProviderAccountStatus,
  ProviderSettingsApiClient,
} from '../../api/provider-settings';

interface ProviderSettingsPageProps {
  projectId: string;
  apiClient?: ProviderSettingsApiClient;
  onBack: () => void;
}

function accountKey(account: ProviderAccountStatus): string {
  return `${account.provider_id}:${account.account_id}`;
}

function statusLabel(state: ProviderAccountStatus['state']): string {
  return state;
}

function callbackError(status: OAuthFlowStatus): string {
  if (status.state === 'expired') return 'Callback OAuth expirado. Inicie a conexão novamente.';
  if (status.state === 'cancelled') return 'Conexão OAuth cancelada.';
  if (status.error_code === 'state_mismatch') return 'Callback inválido: state não corresponde ao fluxo.';
  if (status.error_code === 'redirect_mismatch') return 'Callback inválido: redirect não corresponde ao fluxo.';
  if (status.error_code === 'provider_mismatch') return 'Callback inválido: provider não corresponde ao fluxo.';
  if (status.error_code === 'account_mismatch') return 'Callback inválido: account não corresponde ao fluxo.';
  return 'Callback OAuth inválido ou desatualizado.';
}

export function ProviderSettingsPage({
  projectId,
  apiClient = defaultProviderSettingsApi,
  onBack,
}: ProviderSettingsPageProps) {
  const [accounts, setAccounts] = useState<ProviderAccountStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busyAccount, setBusyAccount] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    setLoading(true);
    apiClient
      .list(projectId)
      .then((result) => {
        if (!active) return;
        setAccounts(result);
        setError(null);
      })
      .catch(() => {
        if (active) setError('Não foi possível carregar os providers.');
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [apiClient, projectId]);

  async function handleConnect(account: ProviderAccountStatus) {
    setBusyAccount(accountKey(account));
    setError(null);
    setMessage(null);
    try {
      const started = await apiClient.startOAuth({
        project_id: projectId,
        provider_id: account.provider_id,
        account_id: account.account_id,
      });
      setPending(started.flow_id);
      setMessage('OAuth pendente. Aguardando retorno validado.');
      await new Promise((resolve) => window.setTimeout(resolve, 25));
      const status = await apiClient.getOAuthStatus({
        project_id: projectId,
        flow_id: started.flow_id,
      });
      if (status.flow_id !== started.flow_id) {
        setError('Callback OAuth desatualizado ou de outro projeto.');
        return;
      }
      if (status.account?.project_id && status.account.project_id !== projectId) {
        setError('Callback OAuth desatualizado para outro projeto.');
        return;
      }
      if (status.state === 'connected' && status.account) {
        setAccounts((current) => current.map((item) =>
          accountKey(item) === accountKey(account) ? status.account as ProviderAccountStatus : item,
        ));
        setMessage('Provider conectado com sucesso.');
        setPending(null);
      } else if (status.state !== 'pending') {
        setError(callbackError(status));
      }
    } catch {
      setError('Não foi possível iniciar a conexão do provider.');
    } finally {
      setBusyAccount(null);
    }
  }

  async function handleDisconnect(account: ProviderAccountStatus) {
    setBusyAccount(accountKey(account));
    setError(null);
    setMessage(null);
    try {
      const result = await apiClient.disconnect({
        project_id: projectId,
        provider_id: account.provider_id,
        account_id: account.account_id,
      });
      setAccounts((current) => current.map((item) =>
        accountKey(item) === accountKey(account) ? result : item,
      ));
      setMessage('Provider revogado com sucesso.');
    } catch {
      setError('Não foi possível desconectar o provider.');
    } finally {
      setBusyAccount(null);
    }
  }

  return (
    <main className="provider-settings-page">
      <header className="provider-settings-header">
        <div>
          <p className="eyebrow">Projeto {projectId}</p>
          <h1>Configurações de providers</h1>
          <p>Conexões são executadas por application services; a interface não acessa credenciais.</p>
        </div>
        <button type="button" onClick={onBack}>Voltar</button>
      </header>

      {error && <div role="alert" className="provider-settings-alert error">{error}</div>}
      {message && <div role="status" className="provider-settings-alert">{message}</div>}
      {loading && <p role="status">Carregando providers...</p>}
      {!loading && !error && accounts.length === 0 && (
        <section className="provider-settings-empty">
          <h2>Nenhum provider conectado</h2>
          <p>Os providers disponíveis aparecerão aqui quando o serviço de contas estiver disponível.</p>
        </section>
      )}
      {!loading && accounts.length > 0 && (
        <section aria-label="Status dos providers" className="provider-account-list">
          {accounts.map((account) => {
            const key = accountKey(account);
            const busy = busyAccount === key;
            return (
              <article className="provider-account-card" key={key}>
                <div className="provider-account-heading">
                  <div>
                    <h2>{account.display_name}</h2>
                    <p>{account.provider_id} · {account.account_id}</p>
                  </div>
                  <span className={`provider-status status-${account.state}`}>{statusLabel(account.state)}</span>
                </div>
                <p className="provider-account-updated">Atualizado em {account.updated_at}</p>
                {account.has_credential_ref && <p className="provider-ref-meta">Credential ref opaco disponível</p>}
                <div className="provider-account-actions">
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => handleConnect(account)}
                  >
                    {busy ? 'Processando...' : `Conectar ${account.display_name}`}
                  </button>
                  {account.state === 'connected' && (
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => handleDisconnect(account)}
                    >
                      {busy ? 'Processando...' : `Desconectar ${account.display_name}`}
                    </button>
                  )}
                </div>
              </article>
            );
          })}
        </section>
      )}
      {pending && <span className="sr-only">Fluxo pendente</span>}
    </main>
  );
}
