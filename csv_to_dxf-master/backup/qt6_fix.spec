# -*- mode: python ; coding: utf-8 -*-

block_cipher = None

a = Analysis(
    ['main.py'],
    pathex=[],
    binaries=[],
    datas=[],
    hiddenimports=['PyQt6.QtCore', 'PyQt6.QtGui', 'PyQt6.QtWidgets'],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

# Qt6プラグインとDLLを確実に含める
qt_plugin_paths = [
    ('platforms/qwindows.dll', 'PyQt6/Qt6/plugins/platforms/qwindows.dll', 'DATA'),
    ('platforms/qdirect2d.dll', 'PyQt6/Qt6/plugins/platforms/qdirect2d.dll', 'DATA'),
    ('platforms', 'PyQt6/Qt6/plugins/platforms', 'DATA'),
    ('imageformats', 'PyQt6/Qt6/plugins/imageformats', 'DATA'),
    ('styles', 'PyQt6/Qt6/plugins/styles', 'DATA'),
]
a.datas += qt_plugin_paths

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name='csv_to_dxf',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=True,
    upx_exclude=[],
    name='csv_to_dxf',
) 