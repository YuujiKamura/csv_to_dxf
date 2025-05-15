# -*- mode: python ; coding: utf-8 -*-

import os
from PyInstaller.utils.hooks import collect_data_files

block_cipher = None

version_file = "version_info.txt"

a = Analysis(
    ['main.py'],
    pathex=[],
    binaries=[],
    datas=[
        ('qt.conf', '.'),
    ] + collect_data_files('PyQt6'),
    hiddenimports=[
        'PyQt6.QtCore',
        'PyQt6.QtGui',
        'PyQt6.QtWidgets',
        'PyQt6.sip',
    ],
    hookspath=[],
    runtime_hooks=[],
    excludes=[
        'tensorflow', 'keras', 'tensorboard',
        'PySide6', 'PyQt5', 'PySide2'
    ],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

# ✅ COLLECT を削除し、EXEが出力を担当する
exe = EXE(
    pyz,
    a.scripts,
    a.binaries + a.zipfiles + a.datas,  # ← 明示的に必要なものを渡す
    name='csv_to_dxf',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    console=False,
    disable_windowed_traceback=True,
    icon='csv_to_dxf_gi.ico',
    version=version_file,
    uac_admin=True,
    onefile=True,  # ✅ これが今度こそ効く
)
