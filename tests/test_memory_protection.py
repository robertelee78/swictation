#!/usr/bin/env python3
"""
Tests for memory protection system.
Tests memory manager, pressure detection, and model offloading.
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src'))

import time
import pytest
import numpy as np
from unittest.mock import Mock, patch, MagicMock

# Import memory manager
from memory_manager import MemoryManager, MemoryPressureLevel, MemoryStatus


class TestMemoryStatus:
    """Test MemoryStatus dataclass"""

    def test_pressure_level_detection(self):
        """Test pressure level categorization"""
        # Normal
        status = MemoryStatus(
            timestamp=time.time(),
            total_mb=8000,
            allocated_mb=6000,  # 75%
            reserved_mb=6200,
            free_mb=2000,
            usage_percent=0.75,
            pressure_level=MemoryPressureLevel.NORMAL
        )
        assert not status.is_critical

        # Warning
        status = MemoryStatus(
            timestamp=time.time(),
            total_mb=8000,
            allocated_mb=6800,  # 85%
            reserved_mb=7000,
            free_mb=1200,
            usage_percent=0.85,
            pressure_level=MemoryPressureLevel.WARNING
        )
        assert not status.is_critical

        # Critical
        status = MemoryStatus(
            timestamp=time.time(),
            total_mb=8000,
            allocated_mb=7200,  # 90%
            reserved_mb=7400,
            free_mb=800,
            usage_percent=0.90,
            pressure_level=MemoryPressureLevel.CRITICAL
        )
        assert status.is_critical

        # Emergency
        status = MemoryStatus(
            timestamp=time.time(),
            total_mb=8000,
            allocated_mb=7800,  # 97.5%
            reserved_mb=7900,
            free_mb=200,
            usage_percent=0.975,
            pressure_level=MemoryPressureLevel.EMERGENCY
        )
        assert status.is_critical


class TestMemoryManager:
    """Test MemoryManager functionality"""

    @pytest.fixture
    def memory_manager(self):
        """Create memory manager with test thresholds"""
        callbacks = {
            'warning': Mock(),
            'critical': Mock(),
            'emergency': Mock(),
            'normal': Mock(),
            'emergency_shutdown': Mock()
        }

        return MemoryManager(
            check_interval=0.1,
            warning_threshold=0.75,
            critical_threshold=0.85,
            emergency_threshold=0.95,
            callbacks=callbacks
        )

    def test_initialization(self, memory_manager):
        """Test memory manager initialization"""
        assert memory_manager.check_interval == 0.1
        assert memory_manager.warning_threshold == 0.75
        assert memory_manager.critical_threshold == 0.85
        assert memory_manager.emergency_threshold == 0.95
        assert not memory_manager.running
        assert len(memory_manager.gpu_models) == 0

    @patch('memory_manager.torch')
    def test_get_memory_status(self, mock_torch, memory_manager):
        """Test memory status retrieval"""
        # Mock GPU availability
        mock_torch.cuda.is_available.return_value = True
        memory_manager.has_gpu = True

        # Mock GPU memory
        mock_props = MagicMock()
        mock_props.total_memory = 8000 * 1e6  # 8GB
        mock_torch.cuda.get_device_properties.return_value = mock_props
        mock_torch.cuda.memory_allocated.return_value = 6000 * 1e6  # 6GB (75%)
        mock_torch.cuda.memory_reserved.return_value = 6200 * 1e6
        mock_torch.cuda.get_device_name.return_value = "Test GPU"

        status = memory_manager.get_memory_status()

        assert status is not None
        assert status.total_mb == 8000
        assert status.allocated_mb == 6000
        assert status.free_mb == 2000
        assert status.usage_percent == 0.75
        assert status.pressure_level == MemoryPressureLevel.NORMAL

    @patch('memory_manager.torch')
    def test_pressure_level_thresholds(self, mock_torch, memory_manager):
        """Test pressure level detection at different thresholds"""
        mock_torch.cuda.is_available.return_value = True
        memory_manager.has_gpu = True

        mock_props = MagicMock()
        mock_props.total_memory = 8000 * 1e6
        mock_torch.cuda.get_device_properties.return_value = mock_props

        # Test WARNING level (75-85%)
        mock_torch.cuda.memory_allocated.return_value = 6400 * 1e6  # 80%
        mock_torch.cuda.memory_reserved.return_value = 6500 * 1e6
        status = memory_manager.get_memory_status()
        assert status.pressure_level == MemoryPressureLevel.WARNING

        # Test CRITICAL level (85-95%)
        mock_torch.cuda.memory_allocated.return_value = 7200 * 1e6  # 90%
        status = memory_manager.get_memory_status()
        assert status.pressure_level == MemoryPressureLevel.CRITICAL

        # Test EMERGENCY level (>95%)
        mock_torch.cuda.memory_allocated.return_value = 7800 * 1e6  # 97.5%
        status = memory_manager.get_memory_status()
        assert status.pressure_level == MemoryPressureLevel.EMERGENCY

    def test_model_registration(self, memory_manager):
        """Test model registration"""
        mock_model = Mock()
        mock_model.cuda = Mock(return_value=mock_model)

        memory_manager.has_gpu = True
        memory_manager.register_model("test_model", mock_model)

        assert "test_model" in memory_manager.gpu_models
        assert memory_manager.gpu_models["test_model"] == mock_model

    @patch('memory_manager.torch')
    @patch('memory_manager.gc')
    def test_handle_warning(self, mock_gc, mock_torch, memory_manager):
        """Test WARNING pressure handling (garbage collection)"""
        mock_torch.cuda.is_available.return_value = True
        memory_manager.has_gpu = True

        # Create mock status
        status = MemoryStatus(
            timestamp=time.time(),
            total_mb=8000,
            allocated_mb=6400,
            reserved_mb=6500,
            free_mb=1600,
            usage_percent=0.80,
            pressure_level=MemoryPressureLevel.WARNING
        )

        # Trigger warning handler
        memory_manager._handle_warning(status)

        # Verify garbage collection was called
        mock_gc.collect.assert_called()
        mock_torch.cuda.empty_cache.assert_called()

    @patch('memory_manager.torch')
    @patch('memory_manager.gc')
    def test_handle_critical(self, mock_gc, mock_torch, memory_manager):
        """Test CRITICAL pressure handling (aggressive cleanup)"""
        mock_torch.cuda.is_available.return_value = True
        memory_manager.has_gpu = True

        status = MemoryStatus(
            timestamp=time.time(),
            total_mb=8000,
            allocated_mb=7200,
            reserved_mb=7400,
            free_mb=800,
            usage_percent=0.90,
            pressure_level=MemoryPressureLevel.CRITICAL
        )

        memory_manager._handle_critical(status)

        # Verify aggressive cleanup (3x)
        assert mock_gc.collect.call_count == 3
        assert mock_torch.cuda.empty_cache.call_count == 3
        mock_torch.cuda.reset_peak_memory_stats.assert_called()

    @patch('memory_manager.torch')
    def test_model_offloading(self, mock_torch, memory_manager):
        """Test model offloading to CPU"""
        mock_torch.cuda.is_available.return_value = True
        memory_manager.has_gpu = True

        # Register mock model
        mock_model = Mock()
        mock_model_cpu = Mock()
        mock_model.cpu = Mock(return_value=mock_model_cpu)
        memory_manager.gpu_models["test_model"] = mock_model

        # Trigger offload
        memory_manager._offload_models_to_cpu()

        # Verify model was offloaded
        assert "test_model" not in memory_manager.gpu_models
        assert "test_model" in memory_manager.offloaded_models
        mock_model.cpu.assert_called()

    @patch('memory_manager.torch')
    def test_model_restoration(self, mock_torch, memory_manager):
        """Test restoring offloaded models to GPU"""
        mock_torch.cuda.is_available.return_value = True
        memory_manager.has_gpu = True

        # Mock GPU status check (60% usage - OK to restore)
        mock_props = MagicMock()
        mock_props.total_memory = 8000 * 1e6
        mock_torch.cuda.get_device_properties.return_value = mock_props
        mock_torch.cuda.memory_allocated.return_value = 4800 * 1e6  # 60%
        mock_torch.cuda.memory_reserved.return_value = 5000 * 1e6

        # Create offloaded model
        mock_model_cpu = Mock()
        mock_model_gpu = Mock()
        mock_model_cpu.cuda = Mock(return_value=mock_model_gpu)
        memory_manager.offloaded_models["test_model"] = mock_model_cpu

        # Restore models
        memory_manager._restore_models_to_gpu()

        # Verify restoration
        assert "test_model" in memory_manager.gpu_models
        assert "test_model" not in memory_manager.offloaded_models
        mock_model_cpu.cuda.assert_called()

    def test_cuda_error_recovery(self, memory_manager):
        """Test CUDA error handling and recovery"""
        # Simulate OOM error
        oom_error = RuntimeError("CUDA out of memory. Tried to allocate 2.00 GiB")

        # First error - should attempt recovery
        result = memory_manager.handle_cuda_error(oom_error)
        assert result == True  # Recovery attempted
        assert memory_manager.cuda_error_count == 1

        # Second error
        result = memory_manager.handle_cuda_error(oom_error)
        assert result == True
        assert memory_manager.cuda_error_count == 2

        # Third error - should trigger fallback
        result = memory_manager.handle_cuda_error(oom_error)
        assert result == False  # Fallback to CPU
        assert memory_manager.cuda_error_count == 3

    def test_error_count_reset(self, memory_manager):
        """Test error counter reset"""
        # Simulate errors
        memory_manager.cuda_error_count = 2

        # Reset
        memory_manager.reset_error_count()

        assert memory_manager.cuda_error_count == 0

    def test_status_report(self, memory_manager):
        """Test status report generation"""
        # Create mock status
        memory_manager.current_status = MemoryStatus(
            timestamp=time.time(),
            total_mb=8000,
            allocated_mb=6000,
            reserved_mb=6200,
            free_mb=2000,
            usage_percent=0.75,
            pressure_level=MemoryPressureLevel.NORMAL
        )

        report = memory_manager.get_status_report()

        assert "Memory Manager Status" in report
        assert "NORMAL" in report
        assert "6000" in report  # allocated
        assert "8000" in report  # total


def test_integration_scenario():
    """Integration test: Simulate memory pressure scenario"""

    # Track callback invocations
    callbacks_called = {
        'warning': False,
        'critical': False,
        'emergency': False,
        'normal': False
    }

    def warning_cb(status):
        callbacks_called['warning'] = True

    def critical_cb(status):
        callbacks_called['critical'] = True

    def emergency_cb(status):
        callbacks_called['emergency'] = True

    def normal_cb(status):
        callbacks_called['normal'] = True

    callbacks = {
        'warning': warning_cb,
        'critical': critical_cb,
        'emergency': emergency_cb,
        'normal': normal_cb
    }

    manager = MemoryManager(
        check_interval=0.1,
        warning_threshold=0.80,
        critical_threshold=0.90,
        emergency_threshold=0.95,
        callbacks=callbacks
    )

    # Simulate pressure changes
    # 1. Start normal
    manager.last_pressure_level = MemoryPressureLevel.NORMAL

    # 2. Warning level
    status = MemoryStatus(
        timestamp=time.time(),
        total_mb=8000,
        allocated_mb=6800,
        reserved_mb=7000,
        free_mb=1200,
        usage_percent=0.85,
        pressure_level=MemoryPressureLevel.WARNING
    )
    manager._handle_pressure_change(status)
    assert callbacks_called['warning']

    # 3. Critical level
    status.usage_percent = 0.92
    status.pressure_level = MemoryPressureLevel.CRITICAL
    manager._handle_pressure_change(status)
    assert callbacks_called['critical']

    # 4. Emergency level
    status.usage_percent = 0.97
    status.pressure_level = MemoryPressureLevel.EMERGENCY
    manager._handle_pressure_change(status)
    assert callbacks_called['emergency']

    # 5. Back to normal
    status.usage_percent = 0.70
    status.pressure_level = MemoryPressureLevel.NORMAL
    manager._handle_pressure_change(status)
    assert callbacks_called['normal']


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
