"""Check the portable desktop wheel before installing or publishing it."""
from email.parser import Parser
from pathlib import Path
import sys
import zipfile

wheels = list(Path(sys.argv[1]).glob('*.whl'))
assert wheels, 'No desktop wheels built'
for wheel in wheels:
    with zipfile.ZipFile(wheel) as archive:
        names = archive.namelist()
        assert not any('.libs/' in n or '.so' in n for n in names), 'Unexpected bundled shared libraries: audit without repair'
        metadata = Parser().parsestr(archive.read(next(n for n in names if n.endswith('/METADATA'))).decode())
        assert metadata['Name'] == 'hypergraph-viz-viewer', metadata['Name']
        assert metadata['Requires-Python'] == '>=3.10'
        binary = [n for n in names if '.data/scripts/' in n and n.rsplit('/', 1)[-1] in ('hyper', 'hyper.exe')]
        assert len(binary) == 1, binary
        data = archive.read(binary[0])
        assert data[:4] in (b'\x7fELF', b'\xcf\xfa\xed\xfe', b'\xfe\xed\xfa\xcf', b'\xca\xfe\xba\xbe') or data[:2] == b'MZ'
        if wheel.name.endswith('win_amd64.whl'):
            import pefile
            pe = pefile.PE(data=data, fast_load=True)
            pe.parse_data_directories(directories=[pefile.DIRECTORY_ENTRY['IMAGE_DIRECTORY_ENTRY_IMPORT']])
            imports = [entry.dll.decode().lower() for entry in pe.DIRECTORY_ENTRY_IMPORT]
            assert not any(name.startswith(('vcruntime', 'msvcp')) for name in imports), f'External Visual C++ runtime required: {imports}'
        else:
            assert (archive.getinfo(binary[0]).external_attr >> 16) & 0o111, 'Executable permission missing'
        assert any('LICENSE-MIT' in n for n in names)
        assert any('LICENSE-APACHE' in n for n in names)
        assert any(n.endswith('/schemas/HIF-LICENSE') for n in names), 'Missing HIF schema license'
        assert any(n.endswith('NOTICE') for n in names)
        assert '-py3-none-' in wheel.name and not wheel.name.endswith('-any.whl'), wheel.name
    assert wheel.stat().st_size < 100 * 1024 * 1024, 'Wheel exceeds default PyPI file limit'
    print(f'Checked desktop wheel {wheel.name} ({wheel.stat().st_size // 1024 // 1024} MiB)')
