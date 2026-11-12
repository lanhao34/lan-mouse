"""Regression test: build scripts must select backends using TARGET, not HOST."""
import os
from pathlib import Path
import subprocess
import tempfile

for source, features in [
    ('input-capture/build.rs', ['layer_shell', 'libei', 'x11']),
    ('input-emulation/build.rs', ['libei', 'x11', 'wlroots', 'remote_desktop_portal']),
]:
    with tempfile.TemporaryDirectory() as temp:
        binary = Path(temp)/'target-cfg'
        command = ['rustc', source, '-o', str(binary)]
        for feature in features: command += ['--cfg', f'feature="{feature}"']
        subprocess.run(command, check=True)
        for family, target in [('windows','windows'), ('unix','macos'), ('unix','linux')]:
            environment = dict(os.environ, CARGO_CFG_TARGET_FAMILY=family, CARGO_CFG_TARGET_OS=target)
            output = subprocess.check_output([str(binary)],env=environment,text=True)
            enabled = [line for line in output.splitlines() if line.startswith('cargo::rustc-cfg=')]
            assert bool(enabled) == (target == 'linux'), (source,target,enabled)
        print(source, 'PASS Windows/macOS/Linux target selection')
