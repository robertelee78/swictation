import { useState } from 'react';
import { LiveSession } from './components/LiveSession';
import { History } from './components/History';
import { Transcriptions } from './components/Transcriptions';
import { LearnedPatterns } from './components/LearnedPatterns';
import { useMetrics } from './hooks/useMetrics';

type Tab = 'live' | 'history' | 'transcriptions' | 'patterns';

function App() {
  const [activeTab, setActiveTab] = useState<Tab>('live');
  const { metrics, transcriptions } = useMetrics();

  return (
    <div className="h-screen flex flex-col bg-background">
      {/* Connection Status */}
      <div className="absolute top-3 right-3 z-50">
        <div
          className={`px-4 py-2 rounded ${
            metrics.connected ? 'bg-success' : 'bg-error'
          } text-white text-xs font-bold`}
        >
          {metrics.connected ? '● LIVE' : '● OFFLINE'}
        </div>
      </div>

      {/* Tab Bar */}
      <div className="flex bg-card border-b border-border">
        <TabButton
          label="Live Session"
          active={activeTab === 'live'}
          onClick={() => setActiveTab('live')}
        />
        <TabButton
          label="History"
          active={activeTab === 'history'}
          onClick={() => setActiveTab('history')}
        />
        <TabButton
          label="Transcriptions"
          active={activeTab === 'transcriptions'}
          onClick={() => setActiveTab('transcriptions')}
        />
        <TabButton
          label="Learned Patterns"
          active={activeTab === 'patterns'}
          onClick={() => setActiveTab('patterns')}
        />
      </div>

      {/* Content Area */}
      <div className="flex-1 overflow-auto">
        {activeTab === 'live' && <LiveSession metrics={metrics} />}
        {activeTab === 'history' && <History />}
        {activeTab === 'transcriptions' && <Transcriptions transcriptions={transcriptions} />}
        {activeTab === 'patterns' && <LearnedPatterns />}
      </div>
    </div>
  );
}

interface TabButtonProps {
  label: string;
  active: boolean;
  onClick: () => void;
}

function TabButton({ label, active, onClick }: TabButtonProps) {
  return (
    <button
      onClick={onClick}
      className={`px-6 py-4 font-mono text-sm transition-colors ${
        active
          ? 'bg-background text-primary border-b-2 border-primary'
          : 'text-muted hover:text-foreground'
      }`}
    >
      {label}
    </button>
  );
}

export default App;
