import React, { useCallback, useEffect, useRef, useState } from 'react';
import { DesktopSchedulerApi, type ScheduledJobInput, type ScheduledJobView, type SchedulerApiClient } from '../api/scheduler';

interface Props { projectId: string; ownerId: string; api?: SchedulerApiClient }

type TriggerKind = 'interval' | 'cron' | 'one_shot';

function inputFor(job: ScheduledJobView): ScheduledJobInput {
  const trigger = job.trigger_kind === 'interval'
    ? { kind: 'interval' as const, seconds: Number(job.trigger_value) }
    : job.trigger_kind === 'one_shot'
      ? { kind: 'one_shot' as const, at_ms: Number(job.trigger_value) }
      : { kind: 'cron' as const, expression: job.trigger_value };
  const target = job.target_kind === 'agent'
    ? { kind: 'agent' as const, id: job.target_id, version: job.target_version }
    : job.target_kind === 'tool'
      ? { kind: 'tool' as const, id: job.target_id, version: job.target_version }
      : { kind: 'workflow' as const, id: job.target_id, version: job.target_version };
  return { project_id: job.project_id, owner_id: job.owner_id, job_id: job.job_id, trigger, target, timezone: job.timezone, concurrency_limit: job.concurrency_limit, missed_run_policy: job.missed_run_policy as ScheduledJobInput['missed_run_policy'], enabled: job.enabled, lifecycle: job.lifecycle };
}

export const AutomationList: React.FC<Props> = ({ projectId, ownerId, api = new DesktopSchedulerApi() }) => {
  const [jobs, setJobs] = useState<ScheduledJobView[]>([]);
  const [kind, setKind] = useState<TriggerKind>('interval');
  const [jobId, setJobId] = useState('');
  const [triggerValue, setTriggerValue] = useState('60');
  const [targetId, setTargetId] = useState('');
  const [targetVersion, setTargetVersion] = useState('1');
  const [timezone, setTimezone] = useState('UTC');
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const requestVersion = useRef(0);

  const load = useCallback(async () => {
    const version = ++requestVersion.current;
    setError(null);
    try {
      const result = await api.list({ project_id: projectId, owner_id: ownerId, limit: 50, offset: 0 });
      if (version === requestVersion.current) setJobs(result);
    } catch (reason) {
      if (version === requestVersion.current) setError(reason instanceof Error ? reason.message : 'Falha ao carregar automações.');
    }
  }, [api, ownerId, projectId]);

  useEffect(() => { void load(); }, [load]);

  const create = async (event: React.FormEvent) => {
    event.preventDefault(); setError(null); setStatus(null);
    const parsedTargetVersion = Number(targetVersion);
    const trigger = kind === 'interval' ? { kind: 'interval' as const, seconds: Number(triggerValue) } : kind === 'one_shot' ? { kind: 'one_shot' as const, at_ms: Number(triggerValue) } : { kind: 'cron' as const, expression: triggerValue.trim() };
    try {
      await api.create({ project_id: projectId, owner_id: ownerId, job_id: jobId.trim(), trigger, target: { kind: 'workflow', id: targetId.trim(), version: parsedTargetVersion }, timezone: timezone.trim(), concurrency_limit: 1, missed_run_policy: 'skip', enabled: true, lifecycle: 'active' });
      setJobId(''); setStatus('Automação criada.'); await load();
    } catch (reason) { setError(reason instanceof Error ? reason.message : 'Falha ao criar automação.'); }
  };

  const toggle = async (job: ScheduledJobView) => {
    setError(null); setStatus(null);
    try { await api.update({ job: { ...inputFor(job), enabled: !job.enabled, lifecycle: job.enabled ? 'disabled' : 'active' }, expected_revision: job.revision }); setStatus(job.enabled ? 'Automação pausada.' : 'Automação reativada.'); await load(); }
    catch (reason) { setError(reason instanceof Error ? reason.message : 'Falha ao atualizar automação.'); }
  };

  return <section aria-label="Automações do projeto" className="automation-list">
    <h3>Automações</h3>
    {status && <p role="status">{status}</p>}
    {error && <p role="alert">{error}</p>}
    <form onSubmit={create} aria-label="Criar automação">
      <label>Identificador <input value={jobId} onChange={(event) => setJobId(event.target.value)} required maxLength={128} /></label>
      <label>Trigger <select value={kind} onChange={(event) => setKind(event.target.value as TriggerKind)}><option value="interval">Intervalo</option><option value="cron">Cron</option><option value="one_shot">Uma vez</option></select></label>
      <label>Valor <input value={triggerValue} onChange={(event) => setTriggerValue(event.target.value)} required aria-label="Valor do trigger" /></label>
      <label>Workflow <input value={targetId} onChange={(event) => setTargetId(event.target.value)} required /></label>
      <label>Versão <input type="number" min="1" value={targetVersion} onChange={(event) => setTargetVersion(event.target.value)} required /></label>
      <label>Timezone <input value={timezone} onChange={(event) => setTimezone(event.target.value)} required /></label>
      <button type="submit">Criar automação</button>
    </form>
    {jobs.length === 0 ? <p role="status">Nenhuma automação encontrada.</p> : <ul aria-label="Jobs agendados">{jobs.map((job) => <li key={job.job_id}><span>{job.job_id} — {job.trigger_kind} — {job.lifecycle} — revisão {job.revision}</span> <button type="button" onClick={() => void toggle(job)}>{job.enabled ? 'Pausar' : 'Reativar'}</button></li>)}</ul>}
  </section>;
};
