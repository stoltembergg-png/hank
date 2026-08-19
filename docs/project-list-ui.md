# Project list UI contract

`ProjectList` receives a `ListProjectsService` instead of accessing storage. It renders
bounded pages and explicit loading, empty, ready and error states. The component does
not create, update or archive projects, and it does not import SQLite, Tauri or provider
code. A stale/unavailable response leaves the previous request isolated and reports a
redacted error state.
