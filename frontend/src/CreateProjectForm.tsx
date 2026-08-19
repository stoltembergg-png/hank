import { FormEvent, useState } from 'react';

export type CreateProjectInput = { name: string };
export type CreateProjectResult =
  | { ok: true; projectId: string }
  | { ok: false; kind: 'validation' | 'conflict' | 'error'; message: string };
export type CreateProjectService = (input: CreateProjectInput) => Promise<CreateProjectResult>;

export function CreateProjectForm({ createProject }: { createProject: CreateProjectService }) {
  const [name, setName] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [result, setResult] = useState<CreateProjectResult | null>(null);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting) return;
    const input = { name: name.trim() };
    if (!input.name) {
      setResult({ ok: false, kind: 'validation', message: 'Project name is required.' });
      return;
    }
    setSubmitting(true);
    setResult(null);
    try {
      setResult(await createProject(input));
    } catch {
      setResult({ ok: false, kind: 'error', message: 'Unable to create project.' });
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <form onSubmit={submit} aria-busy={submitting}>
      <label htmlFor="project-name">Project name</label>
      <input id="project-name" value={name} maxLength={120} onChange={(event) => setName(event.target.value)} disabled={submitting} />
      <button type="submit" disabled={submitting}>{submitting ? 'Creating…' : 'Create project'}</button>
      {result?.ok === true && <p role="status">Project created.</p>}
      {result?.ok === false && <p role="alert">{result.message}</p>}
    </form>
  );
}
