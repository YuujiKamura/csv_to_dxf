# -*- mode: python ; coding: utf-8 -*-

block_cipher = None

a = Analysis(
    ['test_qt.py'],
    pathex=[],
    binaries=[],
    datas=[],
    hiddenimports=[],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

# すべてのPyQt6関連のファイルを含める
import os
from PyInstaller.utils.hooks import collect_data_files

# PyQt6のデータファイルを収集
qt_data = collect_data_files('PyQt6')
a.datas += qt_data

# プラグイン設定ファイルを作成
qt_conf = """[Paths]
Plugins = plugins
"""

with open('qt.conf', 'w') as f:
    f.write(qt_conf)

# qt.confファイルを追加
a.datas += [('qt.conf', 'qt.conf', 'DATA')]

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name='test_qt',
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
    name='test_qt',
) 