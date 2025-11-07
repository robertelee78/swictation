# Swictation UI - Implementation Guide

**Date**: 2025-11-07
**Version**: 0.1.0
**Status**: Architecture Design Complete → Ready for Implementation

This guide provides step-by-step instructions for implementing the Swictation UI based on the architectural design.

## Implementation Phases

### Phase 1: Core Infrastructure ✅ (COMPLETE)
- [x] Project structure created
- [x] Configuration files (Cargo.toml, package.json, tsconfig.json)
- [x] Build system setup (Vite, Tauri)
- [x] Basic Rust backend modules
- [x] Database module with read-only access
- [x] Socket connection module
- [x] Tauri command handlers

### Phase 2: Frontend Foundation (TODO)
- [ ] Type definitions
- [ ] API service layer
- [ ] Custom hooks
- [ ] Layout components
- [ ] Routing setup

### Phase 3: Live Session View (TODO)
- [ ] Real-time metrics chart
- [ ] Session info panel
- [ ] Live transcription feed
- [ ] Connection status indicator

### Phase 4: History View (TODO)
- [ ] Session list with pagination
- [ ] Date range selector
- [ ] Session detail view
- [ ] Historical metrics graphs

### Phase 5: Transcriptions View (TODO)
- [ ] Search interface
- [ ] Results list with highlighting
- [ ] Export functionality
- [ ] Filters and sorting

### Phase 6: Polish & Testing (TODO)
- [ ] Error handling
- [ ] Loading states
- [ ] Empty states
- [ ] Unit tests
- [ ] Integration tests
- [ ] Documentation

---

## Phase 2: Frontend Foundation

### Step 1: Create TypeScript Type Definitions

**File**: `src/types/index.ts`

```typescript
// Session types
export interface SessionSummary {
  id: number;
  start_time: number;
  end_time?: number;
  duration_ms?: number;
  words_dictated: number;
  segments_count: number;
  wpm?: number;
}

// Transcription types
export interface TranscriptionRecord {
  id: number;
  session_id: number;
  text: string;
  timestamp: number;
  latency_ms?: number;
  words: number;
}

// Metrics types
export interface Metrics {
  id: number;
  session_id: number;
  timestamp: number;
  cpu_percent: number;
  memory_mb: number;
  gpu_percent?: number;
  vram_mb?: number;
}

// Stats types
export interface LifetimeStats {
  total_words: number;
  total_characters: number;
  total_sessions: number;
  total_time_minutes: number;
  average_wpm: number;
  average_latency_ms: number;
  best_wpm_value: number;
  best_wpm_session?: number;
}

// Socket types
export interface ConnectionStatus {
  connected: boolean;
  socket_path: string;
}

export interface MetricsUpdate {
  type: 'metrics-update';
  cpu_percent: number;
  memory_mb: number;
  gpu_percent?: number;
  vram_mb?: number;
  wpm?: number;
}

// UI types
export type View = 'live' | 'history' | 'transcriptions';

export interface DateRange {
  start: Date;
  end: Date;
}
```

### Step 2: Create API Service Layer

**File**: `src/services/api.ts`

```typescript
import { invoke } from '@tauri-apps/api/tauri';
import {
  SessionSummary,
  TranscriptionRecord,
  LifetimeStats,
  ConnectionStatus,
} from '../types';

export class ApiService {
  /**
   * Get recent sessions from database
   */
  static async getRecentSessions(limit: number): Promise<SessionSummary[]> {
    return await invoke('get_recent_sessions', { limit });
  }

  /**
   * Get all transcriptions for a session
   */
  static async getSessionDetails(sessionId: number): Promise<TranscriptionRecord[]> {
    return await invoke('get_session_details', { sessionId });
  }

  /**
   * Search transcriptions by text
   */
  static async searchTranscriptions(
    query: string,
    limit: number
  ): Promise<TranscriptionRecord[]> {
    return await invoke('search_transcriptions', { query, limit });
  }

  /**
   * Get lifetime statistics
   */
  static async getLifetimeStats(): Promise<LifetimeStats> {
    return await invoke('get_lifetime_stats');
  }

  /**
   * Toggle recording (send command to daemon)
   */
  static async toggleRecording(): Promise<string> {
    return await invoke('toggle_recording');
  }

  /**
   * Get socket connection status
   */
  static async getConnectionStatus(): Promise<ConnectionStatus> {
    return await invoke('get_connection_status');
  }
}
```

**File**: `src/services/socket.ts`

```typescript
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { MetricsUpdate } from '../types';

export class SocketService {
  private unlistenMetrics?: UnlistenFn;
  private unlistenConnection?: UnlistenFn;

  /**
   * Subscribe to metrics updates
   */
  async subscribeToMetrics(
    callback: (metrics: MetricsUpdate) => void
  ): Promise<void> {
    this.unlistenMetrics = await listen('metrics-update', (event) => {
      callback(event.payload as MetricsUpdate);
    });
  }

  /**
   * Subscribe to connection status changes
   */
  async subscribeToConnection(
    callback: (connected: boolean) => void
  ): Promise<void> {
    this.unlistenConnection = await listen('socket-connected', (event) => {
      callback(event.payload as boolean);
    });
  }

  /**
   * Unsubscribe from all events
   */
  cleanup(): void {
    this.unlistenMetrics?.();
    this.unlistenConnection?.();
  }
}
```

### Step 3: Create Custom Hooks

**File**: `src/hooks/useSocket.ts`

```typescript
import { useState, useEffect } from 'react';
import { SocketService } from '../services/socket';
import { MetricsUpdate } from '../types';

export function useSocket() {
  const [connected, setConnected] = useState(false);
  const [metrics, setMetrics] = useState<MetricsUpdate | null>(null);

  useEffect(() => {
    const socketService = new SocketService();

    socketService.subscribeToMetrics(setMetrics);
    socketService.subscribeToConnection(setConnected);

    return () => socketService.cleanup();
  }, []);

  return { connected, metrics };
}
```

**File**: `src/hooks/useSessions.ts`

```typescript
import { useState, useEffect, useCallback } from 'react';
import { ApiService } from '../services/api';
import { SessionSummary } from '../types';

export function useSessions(limit: number = 50) {
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchSessions = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await ApiService.getRecentSessions(limit);
      setSessions(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch sessions');
    } finally {
      setLoading(false);
    }
  }, [limit]);

  useEffect(() => {
    fetchSessions();
  }, [fetchSessions]);

  return { sessions, loading, error, refetch: fetchSessions };
}
```

**File**: `src/hooks/useTranscriptions.ts`

```typescript
import { useState, useCallback } from 'react';
import { ApiService } from '../services/api';
import { TranscriptionRecord } from '../types';

export function useTranscriptions() {
  const [transcriptions, setTranscriptions] = useState<TranscriptionRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const searchTranscriptions = useCallback(async (query: string, limit: number = 100) => {
    try {
      setLoading(true);
      setError(null);
      const data = await ApiService.searchTranscriptions(query, limit);
      setTranscriptions(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed');
    } finally {
      setLoading(false);
    }
  }, []);

  const getSessionTranscriptions = useCallback(async (sessionId: number) => {
    try {
      setLoading(true);
      setError(null);
      const data = await ApiService.getSessionDetails(sessionId);
      setTranscriptions(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch transcriptions');
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    transcriptions,
    loading,
    error,
    searchTranscriptions,
    getSessionTranscriptions,
  };
}
```

### Step 4: Create Layout Components

**File**: `src/components/Layout/AppLayout.tsx`

```typescript
import React from 'react';

interface AppLayoutProps {
  children: React.ReactNode;
}

export function AppLayout({ children }: AppLayoutProps) {
  return (
    <div className="app-layout">
      <header className="app-header">
        <h1>Swictation</h1>
        <ConnectionIndicator />
      </header>
      <div className="app-content">
        {children}
      </div>
    </div>
  );
}

function ConnectionIndicator() {
  const { connected } = useSocket();

  return (
    <div className={`connection-status ${connected ? 'connected' : 'disconnected'}`}>
      <span className="status-dot"></span>
      <span>{connected ? 'Connected' : 'Disconnected'}</span>
    </div>
  );
}
```

**File**: `src/components/Layout/Sidebar.tsx`

```typescript
import React from 'react';
import { View } from '../../types';

interface SidebarProps {
  activeView: View;
  onViewChange: (view: View) => void;
}

export function Sidebar({ activeView, onViewChange }: SidebarProps) {
  const views: { id: View; label: string; icon: string }[] = [
    { id: 'live', label: 'Live Session', icon: '🎙️' },
    { id: 'history', label: 'History', icon: '📜' },
    { id: 'transcriptions', label: 'Transcriptions', icon: '🔍' },
  ];

  return (
    <nav className="sidebar">
      {views.map((view) => (
        <button
          key={view.id}
          className={`sidebar-item ${activeView === view.id ? 'active' : ''}`}
          onClick={() => onViewChange(view.id)}
        >
          <span className="sidebar-icon">{view.icon}</span>
          <span className="sidebar-label">{view.label}</span>
        </button>
      ))}
    </nav>
  );
}
```

### Step 5: Update App.tsx with Routing

**File**: `src/App.tsx`

```typescript
import React, { useState } from 'react';
import { AppLayout } from './components/Layout/AppLayout';
import { Sidebar } from './components/Layout/Sidebar';
import { LiveSessionView } from './components/LiveSession/LiveSessionView';
import { HistoryView } from './components/History/HistoryView';
import { TranscriptionsView } from './components/Transcriptions/TranscriptionsView';
import { View } from './types';
import './App.css';

function App() {
  const [activeView, setActiveView] = useState<View>('live');

  return (
    <AppLayout>
      <div className="app-container">
        <Sidebar activeView={activeView} onViewChange={setActiveView} />
        <main className="main-content">
          {activeView === 'live' && <LiveSessionView />}
          {activeView === 'history' && <HistoryView />}
          {activeView === 'transcriptions' && <TranscriptionsView />}
        </main>
      </div>
    </AppLayout>
  );
}

export default App;
```

---

## Phase 3: Live Session View

### Step 1: Create LiveSessionView Container

**File**: `src/components/LiveSession/LiveSessionView.tsx`

```typescript
import React from 'react';
import { useSocket } from '../../hooks/useSocket';
import { MetricsChart } from './MetricsChart';
import { SessionInfo } from './SessionInfo';
import { TranscriptionFeed } from './TranscriptionFeed';

export function LiveSessionView() {
  const { connected, metrics } = useSocket();

  if (!connected) {
    return (
      <div className="view-disconnected">
        <h2>Disconnected</h2>
        <p>Waiting for Swictation daemon...</p>
      </div>
    );
  }

  return (
    <div className="live-session-view">
      <div className="metrics-section">
        <MetricsChart />
      </div>
      <div className="info-section">
        <SessionInfo />
      </div>
      <div className="transcription-section">
        <TranscriptionFeed />
      </div>
    </div>
  );
}
```

### Step 2: Create MetricsChart Component

**File**: `src/components/LiveSession/MetricsChart.tsx`

```typescript
import React, { useState, useEffect } from 'react';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend } from 'recharts';
import { useSocket } from '../../hooks/useSocket';

interface DataPoint {
  time: string;
  cpu: number;
  memory: number;
  gpu?: number;
}

export function MetricsChart() {
  const { metrics } = useSocket();
  const [data, setData] = useState<DataPoint[]>([]);

  useEffect(() => {
    if (!metrics) return;

    const now = new Date().toLocaleTimeString();
    const newPoint: DataPoint = {
      time: now,
      cpu: metrics.cpu_percent,
      memory: metrics.memory_mb,
      gpu: metrics.gpu_percent,
    };

    setData(prev => [...prev.slice(-30), newPoint]); // Keep last 30 points
  }, [metrics]);

  return (
    <div className="metrics-chart">
      <h3>System Metrics</h3>
      <LineChart width={600} height={300} data={data}>
        <CartesianGrid strokeDasharray="3 3" />
        <XAxis dataKey="time" />
        <YAxis />
        <Tooltip />
        <Legend />
        <Line type="monotone" dataKey="cpu" stroke="#8884d8" name="CPU %" />
        <Line type="monotone" dataKey="memory" stroke="#82ca9d" name="Memory MB" />
        {data.some(d => d.gpu) && (
          <Line type="monotone" dataKey="gpu" stroke="#ffc658" name="GPU %" />
        )}
      </LineChart>
    </div>
  );
}
```

---

## Implementation Checklist

### Backend (Rust) ✅
- [x] Database connection and queries
- [x] Socket connection and listener
- [x] Tauri command handlers
- [x] Data models
- [ ] Error handling improvements
- [ ] Unit tests

### Frontend Infrastructure
- [ ] Type definitions (`src/types/index.ts`)
- [ ] API service layer (`src/services/api.ts`, `src/services/socket.ts`)
- [ ] Custom hooks (`src/hooks/useSocket.ts`, etc.)
- [ ] Layout components (`src/components/Layout/`)
- [ ] Routing in `App.tsx`

### Live Session View
- [ ] LiveSessionView container
- [ ] MetricsChart component with Recharts
- [ ] SessionInfo panel
- [ ] TranscriptionFeed component
- [ ] Real-time updates

### History View
- [ ] HistoryView container
- [ ] SessionList with pagination
- [ ] DateRangeSelector
- [ ] SessionDetail view
- [ ] Historical MetricsGraph

### Transcriptions View
- [ ] TranscriptionsView container
- [ ] Search interface with debounce
- [ ] TranscriptList with highlighting
- [ ] Export functionality (TXT, JSON, CSV)
- [ ] Filters and sorting

### Testing
- [ ] Unit tests for hooks
- [ ] Component tests
- [ ] Integration tests
- [ ] E2E tests

### Polish
- [ ] Loading states
- [ ] Error states
- [ ] Empty states
- [ ] Responsive design
- [ ] Accessibility (ARIA labels)
- [ ] Keyboard navigation

---

## Development Workflow

### 1. Start Development Environment

```bash
# Terminal 1: Frontend dev server
npm run dev

# Terminal 2: Tauri app
npm run tauri:dev
```

### 2. Test Real-time Connection

```bash
# Terminal 3: Test socket
./scripts/test-socket.sh
```

### 3. Verify Database Access

```bash
# Check database exists
ls -l ~/.local/share/swictation/metrics.db

# Query database
sqlite3 ~/.local/share/swictation/metrics.db "SELECT COUNT(*) FROM sessions;"
```

### 4. Hot Reload

Changes to frontend code trigger instant updates.
Changes to Rust code trigger automatic recompilation and restart.

---

## Next Steps

1. **Implement Phase 2**: Create all foundational files listed above
2. **Test infrastructure**: Verify hooks work with backend
3. **Build Live Session view**: Start with MetricsChart
4. **Iterate**: Add History and Transcriptions views
5. **Polish**: Error handling, loading states, styling
6. **Test**: Unit tests, integration tests
7. **Document**: Update docs with implementation details

---

## Getting Help

- **Architecture questions**: See `docs/ARCHITECTURE.md`
- **API reference**: See `docs/api-reference.md`
- **Socket integration**: See `docs/SOCKET_INTEGRATION.md`
- **Diagrams**: See `docs/ARCHITECTURE_DIAGRAM.md`

---

**Status**: Ready for implementation
**Last Updated**: 2025-11-07
**Next Milestone**: Complete Phase 2 (Frontend Foundation)
