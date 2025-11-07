#!/usr/bin/env python3
"""
Memory pressure manager for Swictation daemon.
Implements pre-emptive monitoring and graceful degradation to prevent OOM crashes.

Features:
- GPU memory pressure detection (80%, 90%, 95% thresholds)
- Automatic model offloading (GPU→CPU)
- CUDA error recovery with fallback
- Emergency shutdown before kernel crash
- Non-blocking memory checks (separate thread)
"""

import threading
import time
import gc
from typing import Optional, Callable, Dict
from enum import Enum
from dataclasses import dataclass


try:
    import torch
    HAS_TORCH = True
except ImportError:
    HAS_TORCH = False


class MemoryPressureLevel(Enum):
    """Memory pressure severity levels"""
    NORMAL = "normal"       # < 80% usage
    WARNING = "warning"     # 80-90% usage
    CRITICAL = "critical"   # 90-95% usage
    EMERGENCY = "emergency" # > 95% usage


@dataclass
class MemoryStatus:
    """Memory status snapshot"""
    timestamp: float
    total_mb: float
    allocated_mb: float
    reserved_mb: float
    free_mb: float
    usage_percent: float
    pressure_level: MemoryPressureLevel

    @property
    def is_critical(self) -> bool:
        """Check if in critical or emergency state"""
        return self.pressure_level in [MemoryPressureLevel.CRITICAL, MemoryPressureLevel.EMERGENCY]


class MemoryManager:
    """
    Memory pressure manager with progressive degradation.

    Monitors GPU/CPU memory and automatically:
    1. Warns at 80% usage
    2. Triggers cleanup at 90%
    3. Offloads models at 95%
    4. Emergency shutdown at 98%
    """

    def __init__(
        self,
        check_interval: float = 2.0,
        warning_threshold: float = 0.80,
        critical_threshold: float = 0.90,
        emergency_threshold: float = 0.95,
        callbacks: Optional[Dict[str, Callable]] = None
    ):
        """
        Initialize memory manager.

        Args:
            check_interval: Seconds between memory checks
            warning_threshold: Warning level (0-1, default 0.80 = 80%)
            critical_threshold: Critical level (0-1, default 0.90 = 90%)
            emergency_threshold: Emergency level (0-1, default 0.95 = 95%)
            callbacks: Optional callbacks for different pressure levels
                      Keys: 'warning', 'critical', 'emergency', 'normal'
        """
        self.check_interval = check_interval
        self.warning_threshold = warning_threshold
        self.critical_threshold = critical_threshold
        self.emergency_threshold = emergency_threshold
        self.callbacks = callbacks or {}

        # State
        self.running = False
        self.monitoring_thread: Optional[threading.Thread] = None
        self.current_status: Optional[MemoryStatus] = None
        self.last_pressure_level = MemoryPressureLevel.NORMAL

        # GPU availability
        self.has_gpu = HAS_TORCH and torch.cuda.is_available()

        # Track models for offloading
        self.gpu_models: Dict[str, any] = {}  # name -> model
        self.offloaded_models: Dict[str, any] = {}  # name -> offloaded model

        # Error recovery state
        self.cuda_error_count = 0
        self.max_cuda_errors = 3  # Fallback to CPU after 3 CUDA errors

    def register_model(self, name: str, model: any):
        """
        Register a model for memory management.

        Args:
            name: Model identifier
            model: Model object (must have .cpu() and .cuda() methods)
        """
        if self.has_gpu and hasattr(model, 'cuda'):
            self.gpu_models[name] = model
            print(f"  MemoryManager: Registered model '{name}' for GPU management")

    def start_monitoring(self):
        """Start background memory monitoring"""
        if self.running:
            return

        if not self.has_gpu:
            print("⚠️  MemoryManager: No GPU available, monitoring disabled")
            return

        self.running = True
        self.monitoring_thread = threading.Thread(
            target=self._monitoring_loop,
            daemon=True,
            name="MemoryMonitor"
        )
        self.monitoring_thread.start()
        print(f"✓ MemoryManager: Monitoring started (check_interval={self.check_interval}s)")

    def stop_monitoring(self):
        """Stop background monitoring"""
        if not self.running:
            return

        self.running = False
        if self.monitoring_thread:
            self.monitoring_thread.join(timeout=5.0)
        print("✓ MemoryManager: Monitoring stopped")

    def get_memory_status(self) -> Optional[MemoryStatus]:
        """
        Get current memory status snapshot.

        Returns:
            MemoryStatus object or None if GPU not available
        """
        if not self.has_gpu:
            return None

        try:
            # Get GPU memory stats
            total = torch.cuda.get_device_properties(0).total_memory
            allocated = torch.cuda.memory_allocated()
            reserved = torch.cuda.memory_reserved()
            free = total - allocated

            total_mb = total / 1e6
            allocated_mb = allocated / 1e6
            reserved_mb = reserved / 1e6
            free_mb = free / 1e6
            usage_percent = allocated / total

            # Determine pressure level
            if usage_percent >= self.emergency_threshold:
                pressure_level = MemoryPressureLevel.EMERGENCY
            elif usage_percent >= self.critical_threshold:
                pressure_level = MemoryPressureLevel.CRITICAL
            elif usage_percent >= self.warning_threshold:
                pressure_level = MemoryPressureLevel.WARNING
            else:
                pressure_level = MemoryPressureLevel.NORMAL

            status = MemoryStatus(
                timestamp=time.time(),
                total_mb=total_mb,
                allocated_mb=allocated_mb,
                reserved_mb=reserved_mb,
                free_mb=free_mb,
                usage_percent=usage_percent,
                pressure_level=pressure_level
            )

            self.current_status = status
            return status

        except Exception as e:
            print(f"⚠️  MemoryManager: Failed to get memory status: {e}")
            return None

    def _monitoring_loop(self):
        """Background monitoring loop"""
        print("  MemoryManager: Monitoring loop started")

        while self.running:
            try:
                status = self.get_memory_status()

                if status:
                    # Check for pressure level changes
                    if status.pressure_level != self.last_pressure_level:
                        self._handle_pressure_change(status)
                        self.last_pressure_level = status.pressure_level

                    # Log periodic status
                    if status.pressure_level != MemoryPressureLevel.NORMAL:
                        print(
                            f"  MemoryManager: [{status.pressure_level.value.upper()}] "
                            f"{status.usage_percent*100:.1f}% "
                            f"({status.allocated_mb:.0f}/{status.total_mb:.0f} MB)"
                        )

                time.sleep(self.check_interval)

            except Exception as e:
                print(f"⚠️  MemoryManager: Monitoring error: {e}")
                import traceback
                traceback.print_exc()
                time.sleep(self.check_interval)

    def _handle_pressure_change(self, status: MemoryStatus):
        """
        Handle memory pressure level changes.

        Args:
            status: Current memory status
        """
        level = status.pressure_level

        print(f"\n⚠️  MEMORY PRESSURE: {level.value.upper()} ({status.usage_percent*100:.1f}%)")

        if level == MemoryPressureLevel.WARNING:
            # 80-90% usage: Trigger garbage collection
            self._handle_warning(status)

        elif level == MemoryPressureLevel.CRITICAL:
            # 90-95% usage: Aggressive cleanup + cache clearing
            self._handle_critical(status)

        elif level == MemoryPressureLevel.EMERGENCY:
            # >95% usage: Offload models to CPU or emergency shutdown
            self._handle_emergency(status)

        elif level == MemoryPressureLevel.NORMAL:
            # Back to normal: Restore models if offloaded
            self._handle_normal(status)

        # Trigger custom callback if registered
        callback_key = level.value
        if callback_key in self.callbacks:
            self.callbacks[callback_key](status)

    def _handle_warning(self, status: MemoryStatus):
        """Handle WARNING pressure level (80-90%)"""
        print("  → Action: Garbage collection")

        # Force garbage collection
        gc.collect()

        if self.has_gpu:
            torch.cuda.empty_cache()

        # Re-check after cleanup
        new_status = self.get_memory_status()
        if new_status:
            freed_mb = status.allocated_mb - new_status.allocated_mb
            print(f"  → Freed {freed_mb:.1f} MB")

    def _handle_critical(self, status: MemoryStatus):
        """Handle CRITICAL pressure level (90-95%)"""
        print("  → Action: Aggressive cleanup")

        # Aggressive garbage collection
        for _ in range(3):
            gc.collect()

        if self.has_gpu:
            # Clear CUDA cache multiple times
            for _ in range(3):
                torch.cuda.empty_cache()

            # Reset peak memory stats
            torch.cuda.reset_peak_memory_stats()

        # Re-check
        new_status = self.get_memory_status()
        if new_status:
            freed_mb = status.allocated_mb - new_status.allocated_mb
            print(f"  → Freed {freed_mb:.1f} MB")

            if new_status.pressure_level == MemoryPressureLevel.CRITICAL:
                print(f"  ⚠️  Still critical after cleanup!")

    def _handle_emergency(self, status: MemoryStatus):
        """Handle EMERGENCY pressure level (>95%)"""
        print("  → Action: EMERGENCY - Offloading models to CPU")

        # First, try aggressive cleanup
        self._handle_critical(status)

        # Re-check after cleanup
        status = self.get_memory_status()
        if status and status.pressure_level == MemoryPressureLevel.EMERGENCY:
            # Still emergency - offload models to CPU
            self._offload_models_to_cpu()

            # Final check
            status = self.get_memory_status()
            if status and status.usage_percent > 0.98:
                # CRITICAL: >98% usage even after offload
                print("  🚨 EMERGENCY SHUTDOWN: Memory >98% after offload!")
                self._emergency_shutdown()

    def _offload_models_to_cpu(self):
        """Offload GPU models to CPU to free memory"""
        if not self.gpu_models:
            print("  → No models to offload")
            return

        print(f"  → Offloading {len(self.gpu_models)} models to CPU...")

        for name, model in self.gpu_models.items():
            try:
                # Move model to CPU
                model_cpu = model.cpu()
                self.offloaded_models[name] = model_cpu

                # Clear GPU memory
                del model
                if self.has_gpu:
                    torch.cuda.empty_cache()

                print(f"    ✓ Offloaded '{name}' to CPU")

            except Exception as e:
                print(f"    ✗ Failed to offload '{name}': {e}")

        # Clear GPU models dict
        self.gpu_models.clear()

        # Final cleanup
        gc.collect()
        if self.has_gpu:
            torch.cuda.empty_cache()

    def _restore_models_to_gpu(self):
        """Restore offloaded models back to GPU"""
        if not self.offloaded_models:
            return

        print(f"  → Restoring {len(self.offloaded_models)} models to GPU...")

        # Check if we have enough GPU memory
        status = self.get_memory_status()
        if status and status.usage_percent > 0.60:
            print(f"  ⚠️  GPU memory at {status.usage_percent*100:.1f}%, deferring restoration")
            return

        for name, model in list(self.offloaded_models.items()):
            try:
                # Move back to GPU
                model_gpu = model.cuda()
                self.gpu_models[name] = model_gpu

                # Remove from offloaded
                del self.offloaded_models[name]

                print(f"    ✓ Restored '{name}' to GPU")

            except RuntimeError as e:
                if "out of memory" in str(e):
                    print(f"    ⚠️  Not enough memory to restore '{name}', keeping on CPU")
                    break
                else:
                    print(f"    ✗ Failed to restore '{name}': {e}")

    def _handle_normal(self, status: MemoryStatus):
        """Handle NORMAL pressure level (<80%)"""
        # If models were offloaded, consider restoring them
        if self.offloaded_models:
            print("  → Memory pressure normal, considering model restoration...")
            self._restore_models_to_gpu()

    def _emergency_shutdown(self):
        """Emergency shutdown to prevent kernel crash"""
        print("\n" + "="*80)
        print("🚨 EMERGENCY SHUTDOWN")
        print("="*80)
        print("GPU memory exhausted (>98% usage)")
        print("Shutting down to prevent kernel crash")
        print("="*80 + "\n")

        # Trigger emergency callback if registered
        if 'emergency_shutdown' in self.callbacks:
            self.callbacks['emergency_shutdown']()

        # Force cleanup
        self.gpu_models.clear()
        self.offloaded_models.clear()
        gc.collect()

        if self.has_gpu:
            torch.cuda.empty_cache()

        # Exit process
        import sys
        sys.exit(1)

    def handle_cuda_error(self, error: Exception) -> bool:
        """
        Handle CUDA errors with automatic recovery.

        Args:
            error: CUDA error exception

        Returns:
            True if recovery successful, False if should fallback to CPU
        """
        self.cuda_error_count += 1

        error_str = str(error).lower()
        is_oom = "out of memory" in error_str

        print(f"⚠️  CUDA Error ({self.cuda_error_count}/{self.max_cuda_errors}): {error}")

        if is_oom:
            # OOM error - trigger emergency cleanup
            print("  → CUDA OOM detected, triggering emergency cleanup...")

            # Aggressive cleanup
            gc.collect()
            if self.has_gpu:
                torch.cuda.empty_cache()
                torch.cuda.ipc_collect()

            # Offload models if available
            if self.gpu_models:
                self._offload_models_to_cpu()

            # Check if recovered
            status = self.get_memory_status()
            if status and status.usage_percent < 0.80:
                print(f"  ✓ Recovered: Memory at {status.usage_percent*100:.1f}%")
                self.cuda_error_count = max(0, self.cuda_error_count - 1)
                return True

        # Check if we should fallback to CPU
        if self.cuda_error_count >= self.max_cuda_errors:
            print(f"  🚨 Max CUDA errors reached ({self.max_cuda_errors})")
            print(f"  → Falling back to CPU mode")
            return False

        return True

    def reset_error_count(self):
        """Reset CUDA error counter after successful operations"""
        if self.cuda_error_count > 0:
            self.cuda_error_count = 0
            print("  ✓ CUDA error count reset")

    def get_status_report(self) -> str:
        """
        Get formatted status report.

        Returns:
            Multi-line status report string
        """
        if not self.current_status:
            return "MemoryManager: No status available"

        status = self.current_status
        lines = [
            "=" * 60,
            "🧠 Memory Manager Status",
            "=" * 60,
            f"Pressure Level: {status.pressure_level.value.upper()}",
            f"GPU Memory: {status.allocated_mb:.1f} / {status.total_mb:.1f} MB ({status.usage_percent*100:.1f}%)",
            f"Free: {status.free_mb:.1f} MB",
            f"Reserved: {status.reserved_mb:.1f} MB",
            "",
            f"Models on GPU: {len(self.gpu_models)}",
            f"Models offloaded: {len(self.offloaded_models)}",
            f"CUDA errors: {self.cuda_error_count}/{self.max_cuda_errors}",
            "=" * 60,
        ]

        return "\n".join(lines)
