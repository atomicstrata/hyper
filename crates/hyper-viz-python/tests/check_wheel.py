"""Validate distributable contents without relying on an editable installation."""
from pathlib import Path
import sys
import zipfile

wheels = list(Path(sys.argv[1]).glob('*.whl'))
assert wheels, 'No wheels built'
for wheel in wheels:
    assert '-cp310-abi3-' in wheel.name, wheel.name
    with zipfile.ZipFile(wheel) as archive:
        names = set(archive.namelist())
        for required in ['hyper_viz/__init__.py', 'hyper_viz/_core.pyi', 'hyper_viz/_viewer.py', 'hyper_viz/_errors.py', 'hyper_viz/py.typed', 'hyper_viz/HIF-LICENSE', 'hyper_viz/LICENSE-MIT', 'hyper_viz/LICENSE-APACHE']:
            assert required in names, f'{wheel.name}: missing {required}'
        assert 'hyper_viz/__init__.pyi' not in names, 'Parent stub hides handwritten API'
        native = [name for name in names if name.startswith('hyper_viz/_core') and name.endswith(('.so','.pyd'))]
        assert len(native) == 1, f'Expected one native extension: {native}'
        assert archive.read('hyper_viz/HIF-LICENSE').startswith(b'MIT License')
        assert not any('__pycache__' in name for name in names), 'Wheel includes bytecode caches'
    print(f'Checked {wheel.name}')
