# Project detail UI contract

`ProjectDetail` consumes load/update/archive application services. Updates carry the
loaded version to prevent lost updates. Archive requires explicit confirmation and
becomes terminal in the UI. Conflict, validation and transport failures are rendered
without exposing storage/provider details. The component does not access SQLite,
filesystem, Tauri or credentials directly.
