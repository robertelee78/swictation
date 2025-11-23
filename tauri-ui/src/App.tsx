import { useState } from 'react';
import { LiveSession } from './components/LiveSession';
import { History } from './components/History';
import { Transcriptions } from './components/Transcriptions';
import { LearnedPatterns } from './components/LearnedPatterns';
import { Analytics } from './components/Analytics';
import { Settings } from './components/Settings';
import { useMetrics } from './hooks/useMetrics';

type Tab = 'live' | 'history' | 'transcriptions' | 'patterns' | 'analytics' | 'settings';

function App() {
  const [activeTab, setActiveTab] = useState<Tab>('live');
  const { metrics, transcriptions } = useMetrics();

  // Helper function to render only the active tab component
  // This prevents React from evaluating all 6 conditional expressions on every render
  const renderActiveTab = () => {
    switch (activeTab) {
      case 'live':
        return <LiveSession metrics={metrics} />;
      case 'history':
        return <History />;
      case 'transcriptions':
        return <Transcriptions transcriptions={transcriptions} />;
      case 'patterns':
        return <LearnedPatterns />;
      case 'analytics':
        return <Analytics />;
      case 'settings':
        return <Settings />;
      default:
        return null;
    }
  };

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
        <TabButton
          label="Analytics"
          active={activeTab === 'analytics'}
          onClick={() => setActiveTab('analytics')}
        />
        <TabButton
          label="Settings"
          active={activeTab === 'settings'}
          onClick={() => setActiveTab('settings')}
        />
      </div>

      {/* Content Area */}
      <div className="flex-1 overflow-auto">
        {renderActiveTab()}
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
