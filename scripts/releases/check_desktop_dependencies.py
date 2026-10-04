"""Reject unused native audio/gamepad dependencies in the desktop executable."""
import subprocess

packages = subprocess.check_output([
    'cargo', 'tree', '--locked', '-p', 'hyper', '--prefix', 'none',
], text=True)
for package in ('bevy_audio', 'bevy_gilrs', 'alsa-sys', 'libudev-sys'):
    assert not any(line.startswith(package + ' ') for line in packages.splitlines()), f'Unexpected desktop dependency: {package}'
print('Desktop dependency graph excludes native audio/gamepad libraries')
