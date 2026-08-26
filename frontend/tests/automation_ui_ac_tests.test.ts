import { describe, expect, it, vi } from 'vitest';
import { DesktopSchedulerApi, type ScheduledJobInput } from '../src/api/scheduler';

const input: ScheduledJobInput = {
  project_id: 'project-a', owner_id: 'owner', job_id: 'job-a',
  trigger: { kind: 'interval', seconds: 60 },
  target: { kind: 'workflow', id: 'workflow-a', version: 1 },
  timezone: 'UTC', concurrency_limit: 1, missed_run_policy: 'skip',
  enabled: true, lifecycle: 'active',
};

describe('automation UI application bridge contract', () => {
  // @spec:AC-1271
  it('sends explicit project-scoped scheduler commands through the Tauri bridge', async () => {
    const invoke = vi.fn().mockResolvedValue([]);
    const api = new DesktopSchedulerApi(invoke);
    await api.list({ project_id: 'project-a', owner_id: 'owner', limit: 50, offset: 0 });
    await api.create(input);
    await api.update({ job: input, expected_revision: 0 });
    expect(invoke).toHaveBeenNthCalledWith(1, 'list_scheduled_jobs', { input: { project_id: 'project-a', owner_id: 'owner', limit: 50, offset: 0 } });
    expect(invoke).toHaveBeenNthCalledWith(2, 'create_scheduled_job', { input });
    expect(invoke).toHaveBeenNthCalledWith(3, 'update_scheduled_job', { input: { job: input, expected_revision: 0 } });
  });

  // @spec:AC-1272
  it('rejects missing explicit trigger and target values before invoking', async () => {
    const invoke = vi.fn();
    const api = new DesktopSchedulerApi(invoke);
    await expect(api.create({ ...input, trigger: undefined as never })).rejects.toThrow('trigger');
    expect(invoke).not.toHaveBeenCalled();
  });
});
