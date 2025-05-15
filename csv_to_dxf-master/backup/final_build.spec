# -*- mode: python ; coding: utf-8 -*-

block_cipher = None

a = Analysis(
    ['main_fixed.py'],
    pathex=[],
    binaries=[],
    datas=[],
    hiddenimports=[
        'PyQt6.QtCore', 
        'PyQt6.QtGui', 
        'PyQt6.QtWidgets',
        'PyQt6.sip',
        'qt6_helper'
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=['tensorflow', 'keras', 'tensorboard'],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

# 必要なDLLとプラグインの追加
import os
from PyInstaller.utils.hooks import collect_data_files, collect_dynamic_libs

# Qt6プラグインを追加
qt_plugins = collect_data_files('PyQt6', include_py_files=True)
a.datas += qt_plugins

# shiboken6のDLLを追加
try:
    import shiboken6
    shiboken_path = os.path.dirname(shiboken6.__file__)
    shiboken_dlls = [(os.path.join('shiboken6', os.path.basename(f)), f, 'DATA') 
                    for f in os.listdir(shiboken_path) 
                    if f.endswith('.dll')]
    a.datas += shiboken_dlls
except (ImportError, FileNotFoundError):
    pass

# PyQt6プラグインの直接指定
platform_plugins = [
    ('platforms/qwindows.dll', 'PyQt6/Qt6/plugins/platforms/qwindows.dll', 'DATA'),
    ('imageformats/qico.dll', 'PyQt6/Qt6/plugins/imageformats/qico.dll', 'DATA'),
    ('imageformats/qjpeg.dll', 'PyQt6/Qt6/plugins/imageformats/qjpeg.dll', 'DATA'),
    ('imageformats/qsvg.dll', 'PyQt6/Qt6/plugins/imageformats/qsvg.dll', 'DATA'),
    ('styles/qwindowsvistastyle.dll', 'PyQt6/Qt6/plugins/styles/qwindowsvistastyle.dll', 'DATA'),
]
a.datas += platform_plugins

# カスタムデータファイルの追加
a.datas += [('qt.conf', 'qt.conf', 'DATA')]

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