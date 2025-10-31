# systemd Integration for Swictation

This document describes the systemd user service integration for automatic daemon startup.

## Overview

Swictation uses a systemd user service to automatically start the daemon when you log into your Sway session. The service handles:
- ✅ Automatic startup with Sway
- ✅ Automatic restart on failure
- ✅ Resource limits (6GB RAM, 200% CPU)
- ✅ Security hardening
- ✅ Logging to systemd journal

## Files

- **Service file**: `config/swictation.service`
- **Installation script**: `scripts/install-systemd-service.sh`
- **Installed location**: `~/.config/systemd/user/swictation.service`

## Installation

### Automatic Installation

```bash
cd /opt/swictation
chmod +x scripts/install-systemd-service.sh
./scripts/install-systemd-service.sh
```

The script will:
1. Create `~/.config/systemd/user/` if needed
2. Backup existing service file (if present)
3. Install the service
4. Reload systemd daemon
5. Enable auto-start
6. Optionally start the service immediately

### Manual Installation

```bash
# Copy service file
cp config/swictation.service ~/.config/systemd/user/

# Reload systemd
systemctl --user daemon-reload

# Enable auto-start
systemctl --user enable swictation.service

# Start now (optional)
systemctl --user start swictation.service
```

## Service Configuration

The service file (`config/swictation.service`) includes:

### Basic Configuration
- **Type**: `simple` (foreground process)
- **ExecStart**: `/usr/bin/python3 /opt/swictation/src/swictationd.py`
- **Restart**: `on-failure` with 5-second delay
- **Output**: Logs to systemd journal

### Resource Limits
- **Memory**: 6GB maximum
- **CPU**: 200% (2 cores)

### Security Hardening
- **PrivateTmp**: Isolated /tmp directory
- **ProtectSystem**: Strict system protection
- **ProtectHome**: Read-only home directory
- **NoNewPrivileges**: Prevents privilege escalation

### Auto-start Configuration
```ini
[Install]
WantedBy=sway-session.target
Also=default.target  # Fallback if sway-session.target doesn't exist
```

## Usage

### Service Control

```bash
# Start the daemon
systemctl --user start swictation.service

# Stop the daemon
systemctl --user stop swictation.service

# Restart the daemon
systemctl --user restart swictation.service

# Check status
systemctl --user status swictation.service

# Enable auto-start (done by installation script)
systemctl --user enable swictation.service

# Disable auto-start
systemctl --user disable swictation.service
```

### Viewing Logs

```bash
# Follow logs in real-time
journalctl --user -u swictation.service -f

# View last 50 lines
journalctl --user -u swictation.service -n 50

# View logs since yesterday
journalctl --user -u swictation.service --since yesterday

# View logs with timestamps
journalctl --user -u swictation.service -o short-precise
```

## Sway Integration

The service is configured to start automatically with Sway via `WantedBy=sway-session.target`.

### How It Works

1. You log into your Sway session
2. systemd starts `sway-session.target`
3. `swictation.service` starts automatically
4. The daemon loads models (~9 seconds)
5. Ready to receive toggle commands (Alt+Shift+d)

### Sway Configuration

Your Sway config should include the keybinding:
```
# Swictation voice dictation toggle
bindsym Mod1+Shift+d exec python3 /opt/swictation/src/swictation_cli.py toggle
```

See `docs/sway-integration.md` for complete Sway setup.

## Troubleshooting

### Service Fails to Start

**Check logs:**
```bash
journalctl --user -u swictation.service -n 50
```

**Common issues:**
1. **CUDA/GPU errors**: Check NVIDIA driver installation
2. **Model download failures**: Check internet connection
3. **Permission errors**: Check file permissions in `/opt/swictation`
4. **Python dependencies**: Reinstall with `pip install -r requirements.txt`

### Service Keeps Restarting

If the service crashes and restarts repeatedly:

```bash
# Check restart count
systemctl --user status swictation.service | grep -i restart

# Disable auto-restart temporarily
systemctl --user edit swictation.service
# Add: [Service]\nRestart=no

# Test manually
python3 /opt/swictation/src/swictationd.py
```

### Service Not Auto-Starting with Sway

**Check if sway-session.target exists:**
```bash
systemctl --user list-units --type=target | grep sway
```

**If not found**, the service will fall back to `default.target`:
```bash
systemctl --user status default.target
```

**Verify service is enabled:**
```bash
systemctl --user is-enabled swictation.service
# Should output: enabled
```

### High Memory Usage

The daemon loads two GPU models:
- Silero VAD: 2.2 MB
- Canary-1B-Flash: 3.6 GB

**Total GPU memory**: ~3.6 GB (fits in 4GB RTX A1000)

If you see memory issues:
```bash
# Check actual memory usage
systemctl --user status swictation.service | grep Mem

# Adjust memory limit in service file if needed
systemctl --user edit swictation.service
# Add: [Service]\nMemoryMax=8G
```

## Uninstallation

To completely remove the systemd service:

```bash
# Stop and disable service
systemctl --user stop swictation.service
systemctl --user disable swictation.service

# Remove service file
rm ~/.config/systemd/user/swictation.service

# Reload daemon
systemctl --user daemon-reload
```

## Logs and Debugging

### Debug Startup Issues

**Run daemon manually to see output:**
```bash
# Stop systemd service first
systemctl --user stop swictation.service

# Run manually
python3 /opt/swictation/src/swictationd.py

# Watch for error messages during startup
```

### Monitor Performance

**Check resource usage:**
```bash
systemctl --user status swictation.service | grep -E "(CPU|Mem)"
```

**Watch in real-time:**
```bash
watch -n 1 'systemctl --user status swictation.service | grep -E "(CPU|Mem)"'
```

## Advanced Configuration

### Custom ExecStart Path

If you installed Swictation in a different location:

```bash
systemctl --user edit swictation.service
```

Add:
```ini
[Service]
ExecStart=
ExecStart=/usr/bin/python3 /your/custom/path/src/swictationd.py
```

### Environment Variables

To add custom environment variables:

```bash
systemctl --user edit swictation.service
```

Add under `[Service]`:
```ini
Environment="CUSTOM_VAR=value"
Environment="CUDA_VISIBLE_DEVICES=0"
```

### Change Resource Limits

```bash
systemctl --user edit swictation.service
```

Add:
```ini
[Service]
MemoryMax=8G
CPUQuota=300%
```

## Integration with Other Init Systems

While this guide focuses on systemd, you can adapt for other init systems:

### OpenRC
Create `/etc/local.d/swictation.start`:
```bash
#!/bin/sh
/usr/bin/python3 /opt/swictation/src/swictationd.py &
```

### runit
Create a run script in `/etc/sv/swictation/run`

### Manual Autostart
Add to `~/.config/sway/config`:
```
exec python3 /opt/swictation/src/swictationd.py
```

---

## References

- [systemd user services documentation](https://www.freedesktop.org/software/systemd/man/systemd.service.html)
- [Sway IPC documentation](https://man.archlinux.org/man/sway-ipc.7.en)
- Swictation README: `README.md`
- Sway integration: `docs/sway-integration.md`
