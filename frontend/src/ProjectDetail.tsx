import { useEffect, useState } from 'react';

export type ProjectDetailData = {
  id: string;
  name: string;
  status: 'active' | 'archived';
  version: number;
  settings: { defaultBudget: number };
};

export type ProjectDetailService = (id: string) => Promise<ProjectDetailData>;
export type UpdateProjectService = (id: string, input: { name: string; version: number }) => Promise<ProjectDetailData | { conflict: true } | { error: true }>;
export type ArchiveProjectService = (id: string, version: number) => Promise<{ ok: true } | { conflict: true } | { error: true }>;

export function ProjectDetail({ projectId, loadProject, updateProject, archiveProject }: {
  projectId: string;
  loadProject: ProjectDetailService;
  updateProject: UpdateProjectService;
  archiveProject: ArchiveProjectService;
}) {
  const [project, setProject] = useState<ProjectDetailData | null>(null);
  const [name, setName] = useState('');
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirmArchive, setConfirmArchive] = useState(false);

  useEffect(() => {
    let active = true;
    setProject(null);
    setMessage(null);
    void loadProject(projectId)
      .then((next) => { if (active) { setProject(next); setName(next.name); } })
      .catch(() => { if (active) setMessage('Unable to load project.'); });
    return () => { active = false; };
  }, [loadProject, projectId]);

  if (!project) return <section aria-busy="true"><p role="status">Loading project…</p>{message && <p role="alert">{message}</p>}</section>;

  const currentProject = project;

  async function save() {
    if (busy) return;
    setBusy(true);
    setMessage(null);
    const result = await updateProject(currentProject.id, { name: name.trim(), version: currentProject.version });
    if ('conflict' in result) setMessage('Project changed elsewhere. Reload before saving.');
    else if ('error' in result) setMessage('Unable to update project.');
    else setProject(result);
    setBusy(false);
  }

  async function archive() {
    if (!confirmArchive || busy) return;
    setBusy(true);
    const result = await archiveProject(currentProject.id, currentProject.version);
    if ('conflict' in result) setMessage('Project changed elsewhere. Reload before archiving.');
    else if ('error' in result) setMessage('Unable to archive project.');
    else setProject({ ...currentProject, status: 'archived' });
    setConfirmArchive(false);
    setBusy(false);
  }

  return (
    <section aria-label="Project detail" aria-busy={busy}>
      <label htmlFor="detail-project-name">Project name</label>
      <input id="detail-project-name" value={name} maxLength={120} disabled={busy || project.status === 'archived'} onChange={(event) => setName(event.target.value)} />
      <p>Status: {project.status}</p>
      {message && <p role="alert">{message}</p>}
      <button type="button" disabled={busy || project.status === 'archived'} onClick={() => void save()}>Save</button>
      <button type="button" disabled={busy || project.status === 'archived'} onClick={() => setConfirmArchive(true)}>Archive</button>
      {confirmArchive && <div role="alertdialog" aria-label="Confirm archive"><p>Archive this project?</p><button type="button" onClick={() => void archive()}>Confirm archive</button><button type="button" onClick={() => setConfirmArchive(false)}>Cancel</button></div>}
    </section>
  );
}
