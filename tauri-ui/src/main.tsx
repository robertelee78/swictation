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

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>
);
