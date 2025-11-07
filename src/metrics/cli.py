"""
CLI interface for metrics display.
"""

import sys
from typing import Optional
from datetime import datetime
import numpy as np

from .database import MetricsDatabase


class MetricsCLI:
    """Command-line interface for displaying metrics."""

    def __init__(self, db_path: str = "~/.local/share/swictation/metrics.db"):
        """
        Initialize CLI with database connection.

        Args:
            db_path: Path to metrics database
        """
        self.db = MetricsDatabase(db_path)

    def show_stats(self, session_id: Optional[int] = None):
        """
        Show detailed statistics for a session.

        Args:
            session_id: Session ID (default: most recent)
        """
        if session_id is None:
            # Get most recent session
            recent = self.db.get_recent_sessions(limit=1)
            if not recent:
                print("No sessions found.", file=sys.stderr)
                return
            session = recent[0]
        else:
            session = self.db.get_session(session_id)
            if not session:
                print(f"Session #{session_id} not found.", file=sys.stderr)
                return

        # Format session statistics
        print("┏" + "━" * 55 + "┓")
        print(f"┃{'📊 SESSION #' + str(session['id']) + ' STATISTICS':^55}┃")
        print("┗" + "━" * 55 + "┛")
        print()

        # Session info
        start_time = datetime.fromtimestamp(session['start_time'])
        print("📅 Session Info")
        print(f"   Date:   {start_time.strftime('%Y-%m-%d %H:%M:%S')}")
        print(f"   ID:     #{session['id']}")
        print()

        # Duration
        if session['duration_s']:
            print("⏱️  Duration")
            print(f"   Total:  {session['duration_s']:.1f}s (wall clock time)")
            print(f"   Active: {session['active_time_s']:.1f}s (dictating, excludes pauses)")
            print(f"   Idle:   {session['pause_time_s']:.1f}s (natural pauses between segments)")
            print()

        # Content
        print("📝 Content")
        print(f"   Words:      {session['words_dictated']}")
        print(f"   Characters: {session['characters_typed']}")
        print(f"   Segments:   {session['segments_processed']}", end="")
        if session['avg_segment_words']:
            print(f" (avg {session['avg_segment_words']:.1f} words/segment)")
        else:
            print()
        print()

        # Performance
        if session['wpm']:
            print("⚡ Performance")
            print(f"   Speed:      {session['wpm']:.0f} wpm (during active dictation)")
            if session['typing_equiv_wpm']:
                speedup = session['wpm'] / session['typing_equiv_wpm']
                print(f"   vs Typing:  {speedup:.1f}x faster (baseline: {session['typing_equiv_wpm']:.0f} wpm typing)")

                # Calculate time saved
                if session['active_time_s']:
                    typing_time = (session['words_dictated'] / session['typing_equiv_wpm']) * 60
                    time_saved = typing_time - session['active_time_s']
                    print(f"   Time Saved: ~{time_saved:.0f} seconds vs typing this amount")
            print()

        # Latency breakdown
        if session['avg_latency_ms']:
            print("⏱️  Latency Breakdown")
            print(f"   Average:    {session['avg_latency_ms']:.0f}ms (silence detected → text injected)")
            print("     ├─ VAD:   ~2000ms (silence threshold, intentional)")

            # Get segments for detailed breakdown
            segments = self.db.get_segments_for_session(session['id'])
            if segments:
                avg_stt = np.mean([s['stt_latency_ms'] for s in segments if s['stt_latency_ms']])
                avg_transform = np.mean([s['transform_latency_us'] for s in segments if s['transform_latency_us']])
                avg_inject = np.mean([s['injection_latency_ms'] for s in segments if s['injection_latency_ms']])

                print(f"     ├─ STT:   {avg_stt:.0f}ms avg (NVIDIA Canary-1B inference)")
                print(f"     ├─ Transform: {avg_transform:.2f}µs avg (PyO3 Rust)")
                print(f"     └─ Inject: {avg_inject:.0f}ms avg (wtype keyboard)")

            print()
            print("   Percentiles:")
            print(f"     ├─ Median (P50): {session['median_latency_ms']:.0f}ms")
            print(f"     ├─ P95:          {session['p95_latency_ms']:.0f}ms")

            if segments:
                max_latency = max([s['total_latency_ms'] for s in segments])
                max_segment = [s for s in segments if s['total_latency_ms'] == max_latency][0]
                print(f"     └─ Max:          {max_latency:.0f}ms (Segment #{segments.index(max_segment) + 1}", end="")
                if max_latency > session['avg_latency_ms'] * 2:
                    print(" - spike detected)")
                else:
                    print(")")
            print()

        # Technical
        if session['gpu_peak_mb'] or session['cpu_peak_percent']:
            print("🔧 Technical")
            if session['gpu_peak_mb']:
                print(f"   GPU Peak:   {session['gpu_peak_mb']:.1f} GB")
                if session['gpu_mean_mb']:
                    print(f"   GPU Avg:    {session['gpu_mean_mb']:.1f} GB")
            if session['cpu_peak_percent']:
                print(f"   CPU Avg:    {session['cpu_mean_percent']:.0f}%")
                print(f"   CPU Peak:   {session['cpu_peak_percent']:.0f}%")
            print()

        # Quality indicators
        if session['transformations_count'] or session['keyboard_actions_count']:
            print("✏️  Quality Indicators")
            if session['transformations_count']:
                print(f"   Transformations: {session['transformations_count']} (voice commands converted)")
            if session['keyboard_actions_count']:
                print(f"   Keyboard Actions: {session['keyboard_actions_count']} (Enter, Backspace, etc.)")
            print()

    def show_history(self, limit: int = 10):
        """
        Show session history table.

        Args:
            limit: Number of recent sessions to show
        """
        sessions = self.db.get_recent_sessions(limit=limit)

        if not sessions:
            print("No sessions found.", file=sys.stderr)
            return

        print("┏" + "━" * 70 + "┓")
        print(f"┃{'📚 SESSION HISTORY (Last ' + str(limit) + ')':^70}┃")
        print("┗" + "━" * 70 + "┛")
        print()

        # Table header
        print("┌" + "─" * 8 + "┬" + "─" * 22 + "┬" + "─" * 7 + "┬" + "─" * 10 + "┬" + "─" * 9 + "┬" + "─" * 12 + "┐")
        print("│ ID     │ Date                 │ Words │ Time     │ WPM     │ Avg Latency│")
        print("├" + "─" * 8 + "┼" + "─" * 22 + "┼" + "─" * 7 + "┼" + "─" * 10 + "┼" + "─" * 9 + "┼" + "─" * 12 + "┤")

        # Calculate averages for highlighting
        wpms = [s['wpm'] for s in sessions if s['wpm']]
        avg_wpm = np.mean(wpms) if wpms else 0

        for session in sessions:
            session_id = f"#{session['id']}"
            date = datetime.fromtimestamp(session['start_time']).strftime('%Y-%m-%d %H:%M')
            words = session['words_dictated']

            # Format time
            if session['active_time_s']:
                time_str = f"{session['active_time_s']:.1f}s"
            else:
                time_str = "—"

            # Format WPM with indicators
            wpm_str = ""
            if session['wpm']:
                wpm_str = f"{session['wpm']:.0f}"
                if session['wpm'] > avg_wpm * 1.2:
                    wpm_str += " 🔥"  # Above average
            else:
                wpm_str = "—"

            # Format latency
            latency_str = ""
            if session['avg_latency_ms']:
                latency_str = f"{session['avg_latency_ms']:.0f}ms"
                if session['avg_latency_ms'] < 350:
                    latency_str += " ⚡"  # Fast
                elif session['avg_latency_ms'] > 500:
                    latency_str += " ⚠️ "  # Slow
            else:
                latency_str = "—"

            # Check for personal best
            words_indicator = ""
            if words == max([s['words_dictated'] for s in sessions]):
                words_indicator = " 🏆"

            print(f"│ {session_id:6} │ {date:20} │ {words:5}{words_indicator:2} │ {time_str:8} │ {wpm_str:7} │ {latency_str:10} │")

        print("└" + "─" * 8 + "┴" + "─" * 22 + "┴" + "─" * 7 + "┴" + "─" * 10 + "┴" + "─" * 9 + "┴" + "─" * 12 + "┘")
        print()
        print("Legend: 🏆 Most words | 🔥 Above average | ⚡ Fast latency | ⚠️  Slow latency")

    def show_summary(self):
        """Show lifetime statistics summary."""
        lifetime = self.db.get_lifetime_stats()

        if not lifetime or lifetime['total_sessions'] == 0:
            print("No lifetime statistics available yet.", file=sys.stderr)
            return

        print("┏" + "━" * 70 + "┓")
        print(f"┃{'🏆 LIFETIME STATISTICS':^70}┃")
        print("┗" + "━" * 70 + "┛")
        print()

        # Totals
        print("📈 Totals (All Time)")
        print(f"   Sessions:   {lifetime['total_sessions']}")
        print(f"   Segments:   {lifetime['total_segments']}")
        print(f"   Words:      {lifetime['total_words']:,} ({lifetime['total_characters']:,} characters)")
        print(f"   Time:       {lifetime['total_time_minutes'] / 60:.1f} hours active dictation")
        print()

        # Performance
        if lifetime['avg_wpm']:
            print("⚡ Performance")
            print(f"   Average Speed:    {lifetime['avg_wpm']:.0f} wpm")
            print(f"   Typing Baseline:  {lifetime['typing_equiv_wpm']:.0f} wpm")
            print(f"   Speedup Factor:   {lifetime['speedup_factor']:.1f}x faster than typing")
            print()
            print(f"   Average Latency:  {lifetime['avg_latency_ms']:.0f}ms")
            print()

        # Trends
        if lifetime['wpm_trend_7day'] or lifetime['latency_trend_7day']:
            print("📊 Trends (Last 7 Days)")
            if lifetime['wpm_trend_7day']:
                trend_str = "📈 (improving!)" if lifetime['wpm_trend_7day'] > 0 else "📉 (declining)"
                print(f"   WPM Trend:      {lifetime['wpm_trend_7day']:+.0f} wpm/week {trend_str}")
            if lifetime['latency_trend_7day']:
                trend_str = "📉 (getting faster!)" if lifetime['latency_trend_7day'] < 0 else "📈 (slowing down)"
                print(f"   Latency Trend:  {lifetime['latency_trend_7day']:+.0f}ms/week {trend_str}")
            print()

        # Productivity
        if lifetime['speedup_factor'] and lifetime['time_saved_minutes']:
            print("💪 Productivity")
            print(f"   vs Typing @ {lifetime['typing_equiv_wpm']:.0f} wpm:   {lifetime['speedup_factor']:.1f}x faster")
            print(f"   Estimated Time Saved: {lifetime['time_saved_minutes'] / 60:.1f} hours")
            print()

            typing_time_minutes = lifetime['total_words'] / lifetime['typing_equiv_wpm']
            print(f"   If you had typed all {lifetime['total_words']:,} words:")
            print(f"     ├─ Typing time:     {typing_time_minutes:.0f} minutes ({typing_time_minutes / 60:.1f} hours)")
            print(f"     └─ Actual time:     {lifetime['total_time_minutes']:.0f} minutes ({lifetime['total_time_minutes'] / 60:.1f} hours)")
            print(f"     └─ Time saved:      {lifetime['time_saved_minutes']:.0f} minutes ({lifetime['time_saved_minutes'] / 60:.1f} hours) 🎉")
            print()

        # Personal bests
        if lifetime['best_wpm_session'] or lifetime['longest_session_id']:
            print("🏅 Personal Bests")
            if lifetime['best_wpm_session'] and lifetime['best_wpm_value']:
                print(f"   Fastest Session:    {lifetime['best_wpm_value']:.0f} wpm (Session #{lifetime['best_wpm_session']})")
            if lifetime['longest_session_id'] and lifetime['longest_session_words']:
                print(f"   Longest Session:    {lifetime['longest_session_words']} words (Session #{lifetime['longest_session_id']})")
            if lifetime['lowest_latency_session'] and lifetime['lowest_latency_ms']:
                print(f"   Lowest Latency:     {lifetime['lowest_latency_ms']:.0f}ms (Session #{lifetime['lowest_latency_session']})")
            print()

        # System health
        print("🔧 System Health")
        print(f"   CUDA Errors:        {lifetime['cuda_errors_total']} total", end="")
        if lifetime['cuda_errors_recovered']:
            print(f" ({lifetime['cuda_errors_recovered']} recovered)")
        else:
            print(" ✓")
        if lifetime['memory_pressure_events']:
            print(f"   Memory Warnings:    {lifetime['memory_pressure_events']} events")
        if lifetime['high_latency_warnings']:
            print(f"   High Latency:       {lifetime['high_latency_warnings']} warnings (> 1000ms segments)")

        # Calculate success rate
        if lifetime['total_segments'] > 0:
            errors = lifetime['cuda_errors_total'] - lifetime['cuda_errors_recovered']
            success_rate = (1 - errors / lifetime['total_segments']) * 100
            print(f"   Uptime:             {success_rate:.1f}% success rate")

    def close(self):
        """Close database connection."""
        self.db.close()


def main():
    """CLI entry point."""
    if len(sys.argv) < 2:
        print("Usage: swictation-metrics <stats|history|summary> [options]")
        sys.exit(1)

    command = sys.argv[1]
    cli = MetricsCLI()

    try:
        if command == "stats":
            session_id = int(sys.argv[2]) if len(sys.argv) > 2 else None
            cli.show_stats(session_id)
        elif command == "history":
            limit = int(sys.argv[2]) if len(sys.argv) > 2 else 10
            cli.show_history(limit)
        elif command == "summary":
            cli.show_summary()
        else:
            print(f"Unknown command: {command}")
            sys.exit(1)
    finally:
        cli.close()


if __name__ == "__main__":
    main()
