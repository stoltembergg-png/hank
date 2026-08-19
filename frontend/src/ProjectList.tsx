import { useEffect, useState } from 'react';

export type ProjectSummary = {
  id: string;
  name: string;
  status: 'active' | 'archived';
};

export type ListProjectsPage = {
  items: ProjectSummary[];
  hasNextPage: boolean;
};

export type ListProjectsService = (page: number, limit: number) => Promise<ListProjectsPage>;

export function ProjectList({ listProjects, pageSize = 20 }: { listProjects: ListProjectsService; pageSize?: number }) {
  const [page, setPage] = useState(0);
  const [result, setResult] = useState<ListProjectsPage | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    setResult(null);
    setError(null);
    void listProjects(page, pageSize)
      .then((next) => { if (active) setResult(next); })
      .catch(() => { if (active) setError('Unable to load projects.'); });
    return () => { active = false; };
  }, [listProjects, page, pageSize]);

  if (error) return <section aria-live="polite"><p role="alert">{error}</p></section>;
  if (!result) return <section aria-busy="true"><p>Loading projects…</p></section>;
  if (result.items.length === 0) return <section aria-live="polite"><p>No projects yet.</p></section>;

  return (
    <section aria-label="Projects">
      <ul>
        {result.items.map((project) => (
          <li key={project.id}>
            <strong>{project.name}</strong>
            <span>{project.status}</span>
          </li>
        ))}
      </ul>
      <nav aria-label="Project pages">
        <button type="button" disabled={page === 0} onClick={() => setPage((current) => current - 1)}>Previous</button>
        <span>Page {page + 1}</span>
        <button type="button" disabled={!result.hasNextPage} onClick={() => setPage((current) => current + 1)}>Next</button>
      </nav>
    </section>
  );
}
