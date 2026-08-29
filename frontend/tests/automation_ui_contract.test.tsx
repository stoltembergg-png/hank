import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { AutomationList } from '../src/components/AutomationList';
import { type SchedulerApiClient } from '../src/api/scheduler';

function api() {
  return {
    list: vi.fn().mockResolvedValue([]),
    create: vi.fn().mockResolvedValue({}),
    update: vi.fn().mockResolvedValue({}),
  };
}

describe('AutomationList', () => {
  // @spec:AC-1272
  it('exposes explicit interval, target, version and timezone fields', async () => {
    const client = api();
    render(<AutomationList projectId="project-a" ownerId="owner" api={client as SchedulerApiClient} />);
    await waitFor(() => expect(client.list).toHaveBeenCalled());
    fireEvent.change(screen.getByLabelText('Identificador'), { target: { value: 'job-a' } });
    fireEvent.change(screen.getByLabelText('Valor do trigger'), { target: { value: '60' } });
    fireEvent.change(screen.getByLabelText('ID do target'), { target: { value: 'workflow-a' } });
    fireEvent.click(screen.getByRole('button', { name: 'Criar automação' }));
    await waitFor(() => expect(client.create).toHaveBeenCalledWith(expect.objectContaining({ project_id: 'project-a', owner_id: 'owner', timezone: 'UTC', trigger: { kind: 'interval', seconds: 60 }, target: { kind: 'workflow', id: 'workflow-a', version: 1 } })));
  });

  // @spec:AC-1272
  it('submits target kind, version, concurrency and missed-run policy explicitly', async () => {
    const client = api();
    render(<AutomationList projectId="project-a" ownerId="owner" api={client as SchedulerApiClient} />);
    await waitFor(() => expect(client.list).toHaveBeenCalled());
    fireEvent.change(screen.getByLabelText('Identificador'), { target: { value: 'job-agent' } });
    fireEvent.change(screen.getByLabelText('Tipo de target'), { target: { value: 'agent' } });
    fireEvent.change(screen.getByLabelText('ID do target'), { target: { value: 'agent-a' } });
    fireEvent.change(screen.getByLabelText('Versão do target'), { target: { value: '2' } });
    fireEvent.change(screen.getByLabelText('Concorrência máxima'), { target: { value: '3' } });
    fireEvent.change(screen.getByLabelText('Política de execuções perdidas'), { target: { value: 'catch_up' } });
    fireEvent.click(screen.getByRole('button', { name: 'Criar automação' }));
    await waitFor(() => expect(client.create).toHaveBeenCalledWith(expect.objectContaining({
      target: { kind: 'agent', id: 'agent-a', version: 2 },
      concurrency_limit: 3,
      missed_run_policy: 'catch_up',
    })));
  });

  // @spec:AC-1274
  it('keeps the list bounded and exposes bridge failures as an accessible alert', async () => {
    const client = api();
    client.list.mockRejectedValue(new Error('bridge unavailable'));
    render(<AutomationList projectId="project-a" ownerId="owner" api={client as SchedulerApiClient} />);
    await waitFor(() => expect(screen.getByRole('alert')).toBeTruthy());
    expect(client.list).toHaveBeenCalledWith({ project_id: 'project-a', owner_id: 'owner', limit: 50, offset: 0 });
    expect(screen.getByRole('alert').textContent).toContain('bridge unavailable');
  });

  // @spec:AC-1273
  it('pauses an enabled job using its current revision', async () => {
    const client = api();
    client.list.mockResolvedValue([{ project_id: 'project-a', job_id: 'job-a', owner_id: 'owner', trigger_kind: 'interval', trigger_value: '60', target_kind: 'workflow', target_id: 'workflow-a', target_version: 1, timezone: 'UTC', concurrency_limit: 1, missed_run_policy: 'skip', enabled: true, lifecycle: 'active', revision: 4 }]);
    render(<AutomationList projectId="project-a" ownerId="owner" api={client as SchedulerApiClient} />);
    await waitFor(() => expect(screen.getByRole('button', { name: 'Pausar' })).toBeTruthy());
    fireEvent.click(screen.getByRole('button', { name: 'Pausar' }));
    await waitFor(() => expect(client.update).toHaveBeenCalledWith(expect.objectContaining({ expected_revision: 4, job: expect.objectContaining({ enabled: false, lifecycle: 'disabled' }) })));
  });
});
