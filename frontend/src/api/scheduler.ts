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

function validateText(value: string, field: string): void {
  if (!value.trim() || value.length > 128 || [...value].some((character) => /\p{Cc}/u.test(character))) {
    throw new Error(`${field} is invalid`);
  }
}

function validate(input: ScheduledJobInput): void {
  validateText(input.project_id, 'project');
  validateText(input.owner_id, 'owner');
  validateText(input.job_id, 'job_id');
  if (!input.trigger || !input.target) throw new Error('trigger and target are required');
  validateText(input.target.id, 'target');
  if (!Number.isSafeInteger(input.target.version) || input.target.version < 1) throw new Error('target version is invalid');
  validateText(input.timezone, 'timezone');
  if (!Number.isSafeInteger(input.concurrency_limit) || input.concurrency_limit < 1 || input.concurrency_limit > 64) throw new Error('concurrency is invalid');
  switch (input.trigger.kind) {
    case 'interval':
      if (!Number.isSafeInteger(input.trigger.seconds) || input.trigger.seconds < 60) throw new Error('interval is invalid');
      break;
    case 'one_shot':
      if (!Number.isSafeInteger(input.trigger.at_ms) || input.trigger.at_ms < 1) throw new Error('one-shot is invalid');
      break;
    case 'cron':
      validateText(input.trigger.expression, 'cron');
      break;
    case 'event':
      validateText(input.trigger.name, 'event');
      break;
    case 'dependency':
      validateText(input.trigger.job_id, 'dependency');
      break;
  }
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
