import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { ErrorBoundary } from './ErrorBoundary';
import './index.css';

// Suppress WebKit inspector localhost errors
window.addEventListener('error', (e) => {
  if (e.message && e.message.includes('localhost')) {
    e.preventDefault();
    e.stopPropagation();
    return false;
  }
});

window.addEventListener('unhandledrejection', (e) => {
  if (e.reason && e.reason.message && e.reason.message.includes('localhost')) {
    e.preventDefault();
    e.stopPropagation();
  }
});

// Prepare app component
const rootElement = document.getElementById('root')!;
const app = (
  <ErrorBoundary>
    <App />
  </ErrorBoundary>
);

// Enable StrictMode only in development for bug detection
// Disable in production to avoid double-invocation overhead
ReactDOM.createRoot(rootElement).render(
  import.meta.env.DEV ? (
    <React.StrictMode>{app}</React.StrictMode>
  ) : (
    app
  )
);
