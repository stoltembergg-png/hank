import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import {
  notifyFrontendReady,
  publishFrontendReady,
  publishFrontendStartupFailure,
} from './api/lifecycle';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);

// Queue the handshake after the root is mounted so the native command proves
// both a loaded WebView and a functioning Tauri IPC route.
queueMicrotask(() => {
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
});
