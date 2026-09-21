# NVIDIA Hibernation Support

## Problem

On laptops with NVIDIA GPUs, hibernation (suspend-to-disk) can cause the GPU to enter a defunct state, resulting in CUDA errors after resume:

```
CUDA error 719: unspecified launch failure
CUDA error 999: unknown error
```

This requires a full system reboot to recover GPU functionality.

## Root Cause

By default, NVIDIA's kernel module does not preserve GPU memory allocations during hibernation. When the system suspends to disk and later resumes, GPU memory state is lost, causing the GPU to become unresponsive.

## Solution

Configure the NVIDIA kernel module to preserve video memory allocations during hibernation by setting the `NVreg_PreserveVideoMemoryAllocations` parameter.

## Manual host configuration

The retired npm installer included a laptop/NVIDIA hibernation helper. Native setup
manages Swictation's user services and artifacts; it does not modify kernel parameters
or initramfs, and must not be run with `sudo` for this purpose. Evaluate the manual
host configuration below against your driver and distribution documentation.


1. **Create modprobe configuration:**

```bash
sudo tee /etc/modprobe.d/nvidia-power-management.conf > /dev/null <<EOF
# NVIDIA Power Management for Laptop Hibernation
# Preserves GPU memory allocations during hibernation/suspend
# Reference: https://download.nvidia.com/XFree86/Linux-x86_64/latest/README/powermanagement.html

options nvidia NVreg_PreserveVideoMemoryAllocations=1 NVreg_TemporaryFilePath=/var/tmp
EOF
```

2. **Update initramfs (distribution-specific):**

**Ubuntu/Debian:**
```bash
sudo update-initramfs -u
```

**Fedora:**
```bash
sudo dracut -f
```

**Arch Linux:**
```bash
sudo mkinitcpio -P
```

3. **Reboot:**
```bash
sudo reboot
```

## Verification

After reboot, verify the configuration:

```bash
# Check kernel parameter
cat /sys/module/nvidia/parameters/PreserveVideoMemoryAllocations
# Should output: 1
```

## Testing

Test your configuration with a hibernation cycle:

1. **Save your work and hibernate:**
   ```bash
   systemctl hibernate
   ```

2. **Resume from hibernation**

3. **Test CUDA/GPU functionality:**
   ```bash
   nvidia-smi
   swictation start
   # Try recording and check for CUDA errors in daemon logs
   ```

4. **Check daemon logs:**
   ```bash
   journalctl --user -u swictation-daemon.service -n 50
   ```

   Look for CUDA errors. If configured correctly, you should see no error 719/999.

## Troubleshooting

### Configuration Not Taking Effect

**Symptom:** Parameter shows 0 after reboot
```bash
cat /sys/module/nvidia/parameters/PreserveVideoMemoryAllocations
# Output: 0
```

**Solutions:**

1. **Verify config file exists:**
   ```bash
   ls -l /etc/modprobe.d/nvidia-power-management.conf
   ```

2. **Check initramfs was updated:**
   ```bash
   # Ubuntu/Debian
   ls -lt /boot/initrd.img-* | head -1

   # Fedora
   ls -lt /boot/initramfs-* | head -1

   # Should show recent modification time
   ```

3. **Manually rebuild initramfs:**
   See "Update initramfs" section above

4. **Check for conflicting configurations:**
   ```bash
   grep -r "NVreg_PreserveVideoMemoryAllocations" /etc/modprobe.d/
   ```

### Still Getting CUDA Errors

1. **Verify NVIDIA driver version:**
   ```bash
   nvidia-smi
   ```
   Power management features require driver version 545.23.06 or newer.

2. **Check systemd hibernation is enabled:**
   ```bash
   systemctl status systemd-hibernate.service
   ```

3. **Enable NVIDIA suspend services:**
   ```bash
   sudo systemctl enable nvidia-suspend.service
   sudo systemctl enable nvidia-hibernate.service
   sudo systemctl enable nvidia-resume.service
   ```

### Desktop Environment Specific

**GNOME Wayland:**
- Some GNOME versions may not support hibernation by default
- Check: `systemctl hibernate` should work

**KDE Plasma:**
- Enable hibernation in Power Management settings
- May require additional swap space configuration

## Distribution-Specific Notes

### Ubuntu 24.04 LTS

- ✅ Full support
- Uses `update-initramfs`
- NVIDIA drivers available via PPA or Ubuntu repositories

### Fedora 39+

- ✅ Full support
- Uses `dracut`
- NVIDIA drivers via RPM Fusion or negativo17

### Arch Linux

- ✅ Full support
- Uses `mkinitcpio`
- NVIDIA drivers in official repositories

### Debian 13+ (Trixie)

- ✅ Full support
- Uses `update-initramfs`
- NVIDIA drivers in non-free repositories

## References

- [NVIDIA Official Documentation](https://download.nvidia.com/XFree86/Linux-x86_64/latest/README/powermanagement.html)
- [NVIDIA Power Management Guide](https://download.nvidia.com/XFree86/Linux-x86_64/580.95.05/README/powermanagement.html)
- [Arch Linux NVIDIA Wiki](https://wiki.archlinux.org/title/NVIDIA/Tips_and_tricks#Preserve_video_memory_after_suspend)

## Distribution boundary

NVIDIA kernel power management is a host administration task. See
[native installation](installation.md) for Swictation setup and repair. The old
JavaScript helper and npm lifecycle hook are retired.
