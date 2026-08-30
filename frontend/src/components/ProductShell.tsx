import React from 'react';
import './ProductShell.css';

export type ProductShellSection =
  | 'overview'
  | 'conversations'
  | 'agents'
  | 'workflows'
  | 'skills'
  | 'memory'
  | 'settings';

export interface ProductShellProps {
  children: React.ReactNode;
  activeSection?: ProductShellSection;
  enabledSections?: readonly ProductShellSection[];
  onSectionChange?: (section: ProductShellSection) => void;
}

type ShellNavigationItem = {
  section: ProductShellSection;
  label: string;
  icon: string;
};

const WORKSPACE_ITEMS: readonly ShellNavigationItem[] = [
  { section: 'overview', label: 'Visão geral', icon: '⌂' },
  { section: 'conversations', label: 'Conversas', icon: '◌' },
  { section: 'agents', label: 'Agents', icon: '✦' },
  { section: 'workflows', label: 'Workflows', icon: '◇' },
  { section: 'skills', label: 'Skills', icon: '✧' },
  { section: 'memory', label: 'Memória', icon: '◉' },
];

const SYSTEM_ITEMS: readonly ShellNavigationItem[] = [
  { section: 'settings', label: 'Configurações', icon: '⚙' },
];

export function ProductShell({
  children,
  activeSection = 'overview',
  enabledSections = ['overview'],
  onSectionChange,
}: ProductShellProps) {
  const enabled = new Set(enabledSections);

  return (
    <div className="product-shell">
      <aside className="product-shell-sidebar" aria-label="Navegação principal">
        <div className="product-shell-brand" aria-label="Hank">
          <span className="product-shell-brand-mark" aria-hidden="true">✦</span>
          <div className="product-shell-brand-copy">
            <span className="product-shell-brand-name">Hank</span>
            <span className="product-shell-brand-subtitle">Desktop workspace</span>
          </div>
        </div>

        <div className="product-shell-workspace-card" aria-label="Workspace ativo">
          <span className="product-shell-workspace-avatar" aria-hidden="true">HD</span>
          <span className="product-shell-workspace-copy">
            <strong>Hank</strong>
            <small>Workspace local</small>
          </span>
          <span className="product-shell-workspace-chevron" aria-hidden="true">⌄</span>
        </div>

        <nav className="product-shell-navigation">
          <NavigationGroup
            label="Workspace"
            items={WORKSPACE_ITEMS}
            activeSection={activeSection}
            enabled={enabled}
            onSectionChange={onSectionChange}
          />
          <NavigationGroup
            label="Sistema"
            items={SYSTEM_ITEMS}
            activeSection={activeSection}
            enabled={enabled}
            onSectionChange={onSectionChange}
          />
        </nav>

        <div className="product-shell-sidebar-footer">
          <div className="product-shell-sidebar-note">
            <span className="product-shell-sidebar-note-dot" aria-hidden="true" />
            <span>Execução segura e local</span>
          </div>
          <div className="product-shell-profile" aria-label="Perfil local">
            <span className="product-shell-profile-avatar" aria-hidden="true">G</span>
            <span className="product-shell-profile-copy">
              <strong>Gabriel</strong>
              <small>Conta local</small>
            </span>
            <span className="product-shell-profile-status" aria-label="Online" />
          </div>
        </div>
      </aside>

      <section className="product-shell-main">
        <header className="product-shell-header" aria-label="Cabeçalho do workspace">
          <div>
            <p className="product-shell-eyebrow">Hank Desktop</p>
            <h1>Workspace</h1>
          </div>
          <div className="product-shell-header-meta">
            <span className="product-shell-header-status">
              <span className="product-shell-header-status-dot" aria-hidden="true" />
              Sessão local
            </span>
            <span className="product-shell-header-divider" aria-hidden="true" />
            <span className="product-shell-header-caption">Privado por padrão</span>
          </div>
        </header>
        <div className="product-shell-content">{children}</div>
      </section>
    </div>
  );
}

function NavigationGroup({
  label,
  items,
  activeSection,
  enabled,
  onSectionChange,
}: {
  label: string;
  items: readonly ShellNavigationItem[];
  activeSection: ProductShellSection;
  enabled: ReadonlySet<ProductShellSection>;
  onSectionChange?: (section: ProductShellSection) => void;
}) {
  return (
    <div className="product-shell-navigation-group">
      <p className="product-shell-navigation-label">{label}</p>
      <div className="product-shell-navigation-items">
        {items.map((item) => {
          const isEnabled = enabled.has(item.section);
          const isActive = activeSection === item.section;

          return (
            <button
              key={item.section}
              type="button"
              className={`product-shell-navigation-item${isActive ? ' is-active' : ''}`}
              disabled={!isEnabled}
              aria-label={item.label}
              aria-current={isActive ? 'page' : undefined}
              title={isEnabled ? item.label : 'Disponível ao integrar o fluxo'}
              onClick={() => onSectionChange?.(item.section)}
            >
              <span className="product-shell-navigation-icon" aria-hidden="true">{item.icon}</span>
              <span>{item.label}</span>
              {!isEnabled && <span className="product-shell-navigation-state">Em integração</span>}
            </button>
          );
        })}
      </div>
    </div>
  );
}
