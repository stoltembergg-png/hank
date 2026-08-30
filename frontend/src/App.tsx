import { useState, useLayoutEffect } from 'react';
import { ProjectList } from './components/ProjectList';
import { ProjectDetailView } from './components/ProjectDetailView';
import { ProductShell, type ProductShellSection } from './components/ProductShell';
import { APP_VERSION } from './version';
import type { ProjectSummary } from './types/project';
import {
  FRONTEND_READY_EVENT,
  FRONTEND_STARTUP_FAILED_EVENT,
  notifyFrontendReady,
  publishFrontendReady,
  publishFrontendStartupFailure,
} from './api/lifecycle';
import './App.css';

function App() {
  const [status, setStatus] = useState<'booting' | 'ready' | 'error'>('booting');
  const [version] = useState<string>(APP_VERSION);
  const [selectedProject, setSelectedProject] = useState<ProjectSummary | null>(null);
  const [activeSection, setActiveSection] = useState<ProductShellSection>('overview');

  useLayoutEffect(() => {
    console.log({ event: 'mount', version, timestamp: new Date().toISOString() });

    const onReady = () => {
      console.log({ event: 'ready', version, timestamp: new Date().toISOString() });
      setStatus('ready');
    };
    const onFailure = (event: Event) => {
      console.error({
        event: 'error',
        version,
        error: event.type,
        timestamp: new Date().toISOString(),
      });
      setStatus('error');
    };

    window.addEventListener(FRONTEND_READY_EVENT, onReady);
    window.addEventListener(FRONTEND_STARTUP_FAILED_EVENT, onFailure);

    // Register listeners before invoking the native command. React's root
    // commit can be deferred, and a microtask in main.tsx could otherwise
    // publish the result before this component is listening (notably after a
    // WebView restart).
    void notifyFrontendReady()
      .then(() => publishFrontendReady())
      .catch((error: unknown) => {
        console.error({
          event: 'frontend_ready_failed',
          error: error instanceof Error ? error.name : 'UnknownError',
          timestamp: new Date().toISOString(),
        });
        publishFrontendStartupFailure(error);
      });

    return () => {
      window.removeEventListener(FRONTEND_READY_EVENT, onReady);
      window.removeEventListener(FRONTEND_STARTUP_FAILED_EVENT, onFailure);
      console.log({ event: 'unmount', version, timestamp: new Date().toISOString() });
    };
  }, [version]);

  const openProject = (project: ProjectSummary) => {
    setSelectedProject(project);
    setActiveSection('overview');
  };

  const closeProject = () => {
    setSelectedProject(null);
    setActiveSection('overview');
  };

  const updateSelectedProject = (project: ProjectSummary) => {
    setSelectedProject(project);
  };

  const enabledSections: readonly ProductShellSection[] = selectedProject
    ? ['overview', 'agents']
    : ['overview'];
  const projectTab = activeSection === 'agents' ? 'agents' : 'overview';

  return (
    <div
      className="app"
      data-hank-frontend-mounted="true"
      data-hank-frontend-ready={status === 'ready' ? 'true' : 'false'}
    >
      <ProductShell
        activeSection={activeSection}
        enabledSections={enabledSections}
        onSectionChange={setActiveSection}
      >
        <div className="app-content-status" aria-label={`Estado da aplicação: ${status}`}>
          <span className={`status ${status}`}>{status}</span>
          <span className="app-version">Version: {version}</span>
        </div>
        {selectedProject ? (
          <ProjectDetailView
            projectId={selectedProject.id}
            initialProject={selectedProject}
            activeTab={projectTab}
            onActiveTabChange={setActiveSection}
            onBack={closeProject}
            onProjectUpdated={updateSelectedProject}
            onProjectArchived={updateSelectedProject}
          />
        ) : (
          <ProjectList onProjectOpen={openProject} />
        )}
      </ProductShell>
    </div>
  );
}

export default App;
