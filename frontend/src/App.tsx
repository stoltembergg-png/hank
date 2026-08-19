import { useState, useEffect } from 'react';
import './App.css';

function App() {
  const [status, setStatus] = useState<'booting' | 'ready' | 'error'>('booting');
  const [version] = useState<string>('0.1.0');

  useEffect(() => {
    console.log({ event: 'mount', version, timestamp: new Date().toISOString() });
    setStatus('booting');
    
    const timer = setTimeout(() => {
      try {
        console.log({ event: 'ready', version, timestamp: new Date().toISOString() });
        setStatus('ready');
      } catch (error) {
        console.error({
          event: 'error',
          version,
          error: error instanceof Error ? error.name : 'unknown',
          timestamp: new Date().toISOString(),
        });
        setStatus('error');
      }
    }, 100);

    return () => {
      clearTimeout(timer);
      console.log({ event: 'unmount', version, timestamp: new Date().toISOString() });
    };
  }, [version]);

  return (
    <div className="app">
      <header>
        <h1>Hank Desktop</h1>
        <span className={`status ${status}`}>{status}</span>
      </header>
      <main>
        <p>Version: {version}</p>
        <p>Frontend workspace initialized and decoupled from Tauri/core.</p>
      </main>
    </div>
  );
}

export default App;