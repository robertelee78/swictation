#!/usr/bin/env python3
"""
Configuration loader for Swictation.
Loads and validates VAD settings from ~/.config/swictation/config.toml
"""

import sys
import tomllib
from pathlib import Path
from dataclasses import dataclass
from typing import Optional


@dataclass
class VADConfig:
    """Voice Activity Detection configuration"""
    threshold: float = 0.5
    silence_duration: float = 2.0


@dataclass
class MetricsWarningsConfig:
    """Metrics warning thresholds"""
    enabled: bool = True
    high_latency_threshold_ms: float = 1000.0
    gpu_memory_threshold_percent: float = 80.0
    degradation_multiplier: float = 1.5
    accuracy_spike_multiplier: float = 3.0


@dataclass
class MetricsCleanupConfig:
    """Metrics database cleanup settings"""
    auto_cleanup_enabled: bool = True
    max_segment_age_days: int = 90
    warn_db_size_mb: int = 100


@dataclass
class MetricsConfig:
    """Performance metrics configuration"""
    enabled: bool = True
    database_path: str = "~/.local/share/swictation/metrics.db"
    show_realtime_feedback: bool = True
    typing_baseline_wpm: float = 40.0
    store_transcription_text: bool = False
    warnings: MetricsWarningsConfig = None
    cleanup: MetricsCleanupConfig = None

    def __post_init__(self):
        if self.warnings is None:
            self.warnings = MetricsWarningsConfig()
        if self.cleanup is None:
            self.cleanup = MetricsCleanupConfig()


@dataclass
class SwictationConfig:
    """Complete Swictation configuration"""
    vad: VADConfig
    metrics: MetricsConfig = None

    def __post_init__(self):
        if self.metrics is None:
            self.metrics = MetricsConfig()


class ConfigLoader:
    """Loads and validates Swictation configuration from TOML file"""

    DEFAULT_CONFIG_PATH = Path.home() / ".config/swictation/config.toml"

    # Validation ranges (from Silero VAD documentation)
    VAD_THRESHOLD_MIN = 0.0
    VAD_THRESHOLD_MAX = 1.0
    SILENCE_DURATION_MIN = 0.3  # Minimum practical value
    SILENCE_DURATION_MAX = 10.0  # Maximum to prevent indefinite waiting

    def __init__(self, config_path: Optional[Path] = None):
        """
        Initialize ConfigLoader.

        Args:
            config_path: Path to config file (default: ~/.config/swictation/config.toml)
        """
        self.config_path = config_path or self.DEFAULT_CONFIG_PATH

    def load(self) -> SwictationConfig:
        """
        Load and validate configuration from file.

        Returns:
            SwictationConfig with validated settings

        Raises:
            SystemExit: On validation errors (with clear error messages)
        """
        # Create default config if file doesn't exist
        if not self.config_path.exists():
            self._create_default_config()
            print(f"✓ Created default config: {self.config_path}", flush=True)
            return SwictationConfig(vad=VADConfig())

        # Load TOML file
        try:
            with open(self.config_path, 'rb') as f:
                config_data = tomllib.load(f)
        except tomllib.TOMLDecodeError as e:
            self._error(f"Invalid TOML syntax in config file:\n{e}\n\nPlease fix {self.config_path} and restart.")
        except Exception as e:
            self._error(f"Failed to read config file:\n{e}\n\nPlease check {self.config_path} and restart.")

        # Validate [vad] section exists
        if 'vad' not in config_data:
            self._error(
                f"Missing [vad] section in config file.\n"
                f"Expected format:\n"
                f"[vad]\n"
                f"threshold = 0.5\n"
                f"silence_duration = 2.0\n\n"
                f"Please fix {self.config_path} and restart."
            )

        vad_section = config_data['vad']

        # Validate required keys exist
        if 'threshold' not in vad_section:
            self._error(
                f"Missing 'threshold' key in [vad] section.\n"
                f"Please add: threshold = 0.5\n"
                f"Fix {self.config_path} and restart."
            )

        if 'silence_duration' not in vad_section:
            self._error(
                f"Missing 'silence_duration' key in [vad] section.\n"
                f"Please add: silence_duration = 2.0\n"
                f"Fix {self.config_path} and restart."
            )

        # Extract values
        try:
            threshold = float(vad_section['threshold'])
        except (ValueError, TypeError):
            self._error(
                f"Invalid VAD threshold value: {vad_section['threshold']}\n"
                f"Must be a number between {self.VAD_THRESHOLD_MIN} and {self.VAD_THRESHOLD_MAX}\n\n"
                f"Please fix {self.config_path} and restart."
            )

        try:
            silence_duration = float(vad_section['silence_duration'])
        except (ValueError, TypeError):
            self._error(
                f"Invalid silence_duration value: {vad_section['silence_duration']}\n"
                f"Must be a number between {self.SILENCE_DURATION_MIN} and {self.SILENCE_DURATION_MAX}\n\n"
                f"Please fix {self.config_path} and restart."
            )

        # Validate ranges
        self._validate_threshold(threshold)
        self._validate_silence_duration(silence_duration)

        # Load metrics config (optional, will use defaults if not present)
        metrics_config = self._load_metrics_config(config_data)

        return SwictationConfig(
            vad=VADConfig(
                threshold=threshold,
                silence_duration=silence_duration
            ),
            metrics=metrics_config
        )

    def _validate_threshold(self, threshold: float):
        """Validate VAD threshold is in valid range"""
        if not (self.VAD_THRESHOLD_MIN <= threshold <= self.VAD_THRESHOLD_MAX):
            self._error(
                f"ERROR: Invalid VAD threshold in config.toml\n"
                f"Found: {threshold}\n"
                f"Valid range: {self.VAD_THRESHOLD_MIN} to {self.VAD_THRESHOLD_MAX}\n"
                f"- 0.0 = most sensitive (more false positives)\n"
                f"- 1.0 = most conservative (may miss soft speech)\n"
                f"- 0.5 = recommended default\n\n"
                f"Please fix {self.config_path} and restart."
            )

    def _validate_silence_duration(self, silence_duration: float):
        """Validate silence duration is in valid range"""
        if not (self.SILENCE_DURATION_MIN <= silence_duration <= self.SILENCE_DURATION_MAX):
            self._error(
                f"ERROR: Invalid silence_duration in config.toml\n"
                f"Found: {silence_duration}\n"
                f"Valid range: {self.SILENCE_DURATION_MIN} to {self.SILENCE_DURATION_MAX} seconds\n"
                f"- 0.3 = minimum (very fast, may cut sentences)\n"
                f"- 2.0 = recommended default (natural pause)\n"
                f"- 3.0+ = slower but more complete\n\n"
                f"Please fix {self.config_path} and restart."
            )

    def _create_default_config(self):
        """Create default configuration file"""
        self.config_path.parent.mkdir(parents=True, exist_ok=True)

        default_content = """# Swictation Configuration
# VAD (Voice Activity Detection) Settings

[vad]
# Speech detection threshold (0.0-1.0)
# Controls sensitivity for detecting speech vs noise
# - 0.0 = most sensitive (everything is speech, many false positives)
# - 0.5 = balanced (recommended default)
# - 1.0 = most conservative (only confident speech, may miss quiet speech)
threshold = 0.5

# Silence duration in seconds before processing text
# How long to wait after speech ends before transcribing
# - Lower = faster response, may cut off sentences
# - Higher = more complete sentences, slower response
# - Common range: 0.5-3.0 seconds
silence_duration = 2.0
"""

        with open(self.config_path, 'w') as f:
            f.write(default_content)

    def _load_metrics_config(self, config_data: dict) -> MetricsConfig:
        """
        Load metrics configuration from config data.
        All metrics settings are optional - uses defaults if not present.
        """
        # If no [metrics] section, return defaults
        if 'metrics' not in config_data:
            return MetricsConfig()

        metrics_section = config_data['metrics']

        # Load main metrics settings (all optional)
        enabled = metrics_section.get('enabled', True)
        database_path = metrics_section.get('database_path', '~/.local/share/swictation/metrics.db')
        show_realtime_feedback = metrics_section.get('show_realtime_feedback', True)
        typing_baseline_wpm = float(metrics_section.get('typing_baseline_wpm', 40.0))
        store_transcription_text = metrics_section.get('store_transcription_text', False)

        # Load warnings config (optional subsection)
        warnings_config = MetricsWarningsConfig()
        if 'warnings' in metrics_section:
            warnings_section = metrics_section['warnings']
            warnings_config = MetricsWarningsConfig(
                enabled=warnings_section.get('enabled', True),
                high_latency_threshold_ms=float(warnings_section.get('high_latency_threshold_ms', 1000.0)),
                gpu_memory_threshold_percent=float(warnings_section.get('gpu_memory_threshold_percent', 80.0)),
                degradation_multiplier=float(warnings_section.get('degradation_multiplier', 1.5)),
                accuracy_spike_multiplier=float(warnings_section.get('accuracy_spike_multiplier', 3.0))
            )

        # Load cleanup config (optional subsection)
        cleanup_config = MetricsCleanupConfig()
        if 'cleanup' in metrics_section:
            cleanup_section = metrics_section['cleanup']
            cleanup_config = MetricsCleanupConfig(
                auto_cleanup_enabled=cleanup_section.get('auto_cleanup_enabled', True),
                max_segment_age_days=int(cleanup_section.get('max_segment_age_days', 90)),
                warn_db_size_mb=int(cleanup_section.get('warn_db_size_mb', 100))
            )

        return MetricsConfig(
            enabled=enabled,
            database_path=database_path,
            show_realtime_feedback=show_realtime_feedback,
            typing_baseline_wpm=typing_baseline_wpm,
            store_transcription_text=store_transcription_text,
            warnings=warnings_config,
            cleanup=cleanup_config
        )

    def _error(self, message: str):
        """Print error message and exit"""
        print(f"\n{message}\n", file=sys.stderr, flush=True)
        sys.exit(1)
