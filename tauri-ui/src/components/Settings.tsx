import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface DaemonConfig {
  socket_path: string;
  vad_model_path: string;
  vad_min_silence: number;
  vad_min_speech: number;
  vad_max_speech: number;
  vad_threshold: number;
  stt_model_override: string;
  stt_0_6b_model_path: string;
  stt_1_1b_model_path: string;
  num_threads: number | null;
  audio_device_index: number | null;
  hotkeys: {
    toggle: string;
    push_to_talk: string;
  };
  phonetic_threshold: number;
}

export function Settings() {
  const [config, setConfig] = useState<DaemonConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  useEffect(() => {
    loadConfig();
  }, []);

  const loadConfig = async () => {
    try {
      setLoading(true);
      setError(null);
      const cfg = await invoke<DaemonConfig>('get_daemon_config');
      setConfig(cfg);
    } catch (err) {
      setError(`Failed to load config: ${err}`);
      console.error('Config load error:', err);
    } finally {
      setLoading(false);
    }
  };

  const savePhoneticThreshold = async (threshold: number) => {
    try {
      setSaving(true);
      setError(null);
      setSuccessMessage(null);

      await invoke('update_phonetic_threshold', { threshold });

      setSuccessMessage('✓ Threshold saved! Restart daemon to apply changes.');

      // Clear success message after 3 seconds
      setTimeout(() => setSuccessMessage(null), 3000);

      // Reload config to confirm
      await loadConfig();
    } catch (err) {
      setError(`Failed to save threshold: ${err}`);
      console.error('Save error:', err);
    } finally {
      setSaving(false);
    }
  };

  const handleThresholdChange = (value: number) => {
    if (config) {
      setConfig({ ...config, phonetic_threshold: value });
    }
  };

  const handleThresholdSave = () => {
    if (config) {
      savePhoneticThreshold(config.phonetic_threshold);
    }
  };

  if (loading) {
    return (
      <div className="p-8">
        <div className="text-muted">Loading configuration...</div>
      </div>
    );
  }

  if (!config) {
    return (
      <div className="p-8">
        <div className="text-error">{error || 'Failed to load configuration'}</div>
        <button
          onClick={loadConfig}
          className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded hover:bg-primary/90"
        >
          Retry
        </button>
      </div>
    );
  }

  const getThresholdLabel = (value: number) => {
    if (value < 0.2) return 'Very Strict';
    if (value < 0.35) return 'Moderate';
    if (value < 0.6) return 'Relaxed';
    return 'Very Fuzzy';
  };

  return (
    <div className="p-8 max-w-4xl mx-auto">
      <h1 className="text-2xl font-bold text-foreground mb-6">Settings</h1>

      {error && (
        <div className="mb-6 p-4 bg-error/10 border border-error rounded text-error">
          {error}
        </div>
      )}

      {successMessage && (
        <div className="mb-6 p-4 bg-success/10 border border-success rounded text-success">
          {successMessage}
        </div>
      )}

      {/* Phonetic Threshold Section */}
      <div className="bg-card border border-border rounded-lg p-6 mb-6">
        <h2 className="text-xl font-bold text-foreground mb-4">
          Learned Corrections
        </h2>

        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-foreground mb-2">
              Phonetic Matching Threshold
            </label>
            <p className="text-sm text-muted mb-4">
              Controls how strictly phonetic corrections are matched. Lower values require
              closer matches, higher values allow more fuzzy matching.
            </p>
          </div>

          <div className="space-y-3">
            <div className="flex items-center gap-4">
              <span className="text-sm text-muted min-w-[80px]">
                Strict (0.0)
              </span>
              <input
                type="range"
                min="0"
                max="1"
                step="0.05"
                value={config.phonetic_threshold}
                onChange={(e) => handleThresholdChange(parseFloat(e.target.value))}
                className="flex-1 h-2 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
              />
              <span className="text-sm text-muted min-w-[80px] text-right">
                Fuzzy (1.0)
              </span>
            </div>

            <div className="flex items-center justify-between">
              <div className="text-sm">
                <span className="text-muted">Current: </span>
                <span className="font-mono text-foreground">
                  {config.phonetic_threshold.toFixed(2)}
                </span>
                <span className="ml-2 text-primary font-medium">
                  ({getThresholdLabel(config.phonetic_threshold)})
                </span>
              </div>
              <button
                onClick={handleThresholdSave}
                disabled={saving}
                className="px-4 py-2 bg-primary text-primary-foreground rounded hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {saving ? 'Saving...' : 'Save Threshold'}
              </button>
            </div>
          </div>

          <div className="mt-4 p-4 bg-muted/30 rounded">
            <p className="text-sm text-muted">
              <strong>Note:</strong> Changes require daemon restart to take effect.
              The daemon watches the config file and will reload on next restart.
            </p>
          </div>
        </div>
      </div>

      {/* Future Settings Sections */}
      <div className="bg-card border border-border rounded-lg p-6 opacity-50">
        <h2 className="text-xl font-bold text-foreground mb-4">
          Hotkeys (Coming Soon)
        </h2>
        <p className="text-sm text-muted">
          Hotkey configuration will be available in a future update.
        </p>
      </div>
    </div>
  );
}
