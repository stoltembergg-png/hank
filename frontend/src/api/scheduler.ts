export type TriggerInput =
  | { kind: 'one_shot'; at_ms: number }
  | { kind: 'interval'; seconds: number }
  | { kind: 'cron'; expression: string }
  | { kind: 'event'; name: string }
  | { kind: 'dependency'; job_id: string };
export type TargetInput =
  | { kind: 'workflow'; id: string; version: number }
  | { kind: 'agent'; id: string; version: number }
  | { kind: 'tool'; id: string; version: number };
export type MissedRunPolicyInput = 'skip' | 'catch_up' | 'pause';
export interface ScheduledJobInput {
  project_id: string;
  owner_id: string;
  job_id: string;
  trigger: TriggerInput;
  target: TargetInput;
  timezone: string;
  concurrency_limit: number;
  missed_run_policy: MissedRunPolicyInput;
  enabled?: boolean;
  lifecycle?: string;
  expires_at_ms?: number;
}
export interface ListScheduledJobsInput { project_id: string; owner_id: string; limit?: number; offset?: number }
export interface UpdateScheduledJobInput { job: ScheduledJobInput; expected_revision: number }
export interface ScheduledJobView extends ScheduledJobInput {
  trigger_kind: string;
  trigger_value: string;
  target_kind: string;
  target_id: string;
  target_version: number;
  revision: number;
}
type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

function validate(input: ScheduledJobInput): void {
  if (!input.project_id || !input.owner_id || !input.job_id) throw new Error('project, owner and job_id are required');
  if (!input.trigger || !input.target) throw new Error('trigger and target are required');
  if (!input.timezone || input.concurrency_limit < 1 || input.concurrency_limit > 64) throw new Error('timezone and bounded concurrency are required');
}

function desktopInvoke(): Invoke {
  const bridge = window as unknown as { __TAURI_INTERNALS__?: { invoke?: Invoke } };
  const invoke = bridge.__TAURI_INTERNALS__?.invoke;
  if (!invoke) throw new Error('Scheduler desktop bridge is unavailable');
  return invoke;
}

export interface SchedulerApiClient {
  list(input: ListScheduledJobsInput): Promise<ScheduledJobView[]>;
  create(input: ScheduledJobInput): Promise<ScheduledJobView>;
  update(input: UpdateScheduledJobInput): Promise<ScheduledJobView>;
}

export class DesktopSchedulerApi implements SchedulerApiClient {
  private readonly invoke?: Invoke;
  constructor(invoke?: Invoke) { this.invoke = invoke; }
  private getInvoker(): Invoke { return this.invoke ?? desktopInvoke(); }
  list(input: ListScheduledJobsInput): Promise<ScheduledJobView[]> {
    if (!input.project_id || !input.owner_id) return Promise.reject(new Error('project and owner are required'));
    return this.getInvoker()<ScheduledJobView[]>('list_scheduled_jobs', { input });
  }
  async create(input: ScheduledJobInput): Promise<ScheduledJobView> {
    validate(input);
    return this.getInvoker()<ScheduledJobView>('create_scheduled_job', { input });
  }
  async update(input: UpdateScheduledJobInput): Promise<ScheduledJobView> {
    validate(input.job);
    if (!Number.isInteger(input.expected_revision) || input.expected_revision < 0) throw new Error('expected_revision is required');
    return this.getInvoker()<ScheduledJobView>('update_scheduled_job', { input });
  }
}
