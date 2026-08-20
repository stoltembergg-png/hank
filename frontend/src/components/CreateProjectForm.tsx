import React, { useState } from 'react';
import { ProjectApiClient, defaultProjectApi } from '../api/projects';
import { ProjectSummary } from '../types/project';
import './CreateProjectForm.css';

export interface CreateProjectFormProps {
  apiClient?: ProjectApiClient;
  onSuccess?: (project: ProjectSummary) => void;
  onCancel?: () => void;
}

export const CreateProjectForm: React.FC<CreateProjectFormProps> = ({
  apiClient = defaultProjectApi,
  onSuccess,
  onCancel,
}) => {
  const [name, setName] = useState('');
  const [owner, setOwner] = useState('');
  const [description, setDescription] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (isSubmitting) return;

    const trimmedName = name.trim();
    const trimmedOwner = owner.trim();
    const trimmedDesc = description.trim();

    if (!trimmedName) {
      setError('O nome do projeto é obrigatório.');
      return;
    }

    if (trimmedName.length > 100) {
      setError('O nome do projeto deve ter no máximo 100 caracteres.');
      return;
    }

    if (!trimmedOwner) {
      setError('O responsável pelo projeto é obrigatório.');
      return;
    }

    setIsSubmitting(true);
    setError(null);

    try {
      const response = await apiClient.create({
        name: trimmedName,
        owner: trimmedOwner,
        description: trimmedDesc ? trimmedDesc : undefined,
        correlation_id: `req_${Date.now().toString(36)}`,
      });

      setName('');
      setOwner('');
      setDescription('');
      onSuccess?.(response.project);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Erro ao criar o projeto.');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="create-project-card" aria-label="Criar Novo Projeto">
      <div className="create-project-header">
        <h3>Novo Projeto</h3>
        {onCancel && (
          <button
            type="button"
            className="btn-close"
            onClick={onCancel}
            disabled={isSubmitting}
            aria-label="Fechar formulário"
          >
            &times;
          </button>
        )}
      </div>

      {error && (
        <div className="create-project-error" role="alert">
          {error}
        </div>
      )}

      <form onSubmit={handleSubmit} className="create-project-form" noValidate>
        <div className="form-group">
          <label htmlFor="project-name-input">
            Nome do Projeto <span className="required-star">*</span>
          </label>
          <input
            id="project-name-input"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            disabled={isSubmitting}
            maxLength={100}
            placeholder="Ex: Hank Agent Core"
            required
            aria-required="true"
          />
          <span className="form-hint">Máximo de 100 caracteres</span>
        </div>

        <div className="form-group">
          <label htmlFor="project-owner-input">
            Responsável <span className="required-star">*</span>
          </label>
          <input
            id="project-owner-input"
            type="text"
            value={owner}
            onChange={(e) => setOwner(e.target.value)}
            disabled={isSubmitting}
            maxLength={100}
            placeholder="Ex: dev@hank.local"
            required
            aria-required="true"
          />
        </div>

        <div className="form-group">
          <label htmlFor="project-desc-input">Descrição</label>
          <textarea
            id="project-desc-input"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            disabled={isSubmitting}
            maxLength={500}
            rows={3}
            placeholder="Descrição opcional dos objetivos do projeto"
          />
        </div>

        <div className="form-actions">
          {onCancel && (
            <button
              type="button"
              className="btn-cancel"
              onClick={onCancel}
              disabled={isSubmitting}
            >
              Cancelar
            </button>
          )}
          <button
            type="submit"
            className="btn-submit"
            disabled={isSubmitting || !name.trim() || !owner.trim()}
          >
            {isSubmitting ? 'Criando...' : 'Criar Projeto'}
          </button>
        </div>
      </form>
    </div>
  );
};
