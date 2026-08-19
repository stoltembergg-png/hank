# Create Project UI contract

`CreateProjectForm` accepts only an allowlisted bounded project name and calls an
injected `CreateProjectService`. It validates empty input, prevents duplicate submits,
disables controls while pending, and renders success, validation, conflict and generic
error states. It does not construct persistence entities, access storage, execute paths
or handle secrets.
