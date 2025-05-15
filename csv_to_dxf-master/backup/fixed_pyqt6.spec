# -*- mode: python ; coding: utf-8 -*-

block_cipher = None

a = Analysis(
    ['main.py'],
    pathex=[],
    binaries=[],
    datas=[],
    hiddenimports=[
        'PyQt6.QtCore',
        'PyQt6.QtGui',
        'PyQt6.QtWidgets',
        'PyQt6.sip'
    ],
    hookspath=[],
    hooksconfig={
        'PyQt6': {
            'gui': True
        }
    },
    runtime_hooks=[],
    # 他のQtバインディングを明示的に除外
    excludes=[
        'PySide6',
        'PySide2',
        'PyQt5',
        'shiboken6',
        'shiboken2',
        'PySide',
        'PyQt4',
        'tensorflow',
        'keras',
        'tensorboard'
    ],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

# PyQt6プラグインを明示的に追加（パスが存在する場合のみ）
import os
import sys
from PyInstaller.utils.hooks import collect_system_data_files
from PyInstaller.utils.hooks import collect_data_files

# 明示的にPyQt6のプラグインのみを収集
qt_plugins = []
try:
    import PyQt6
    pyqt6_path = os.path.dirname(PyQt6.__file__)
    
    # プラグインの明示的なパス
    qt6_plugin_paths = [
        os.path.join(pyqt6_path, 'Qt6', 'plugins'),
        os.path.join(pyqt6_path, 'plugins'),
    ]
    
    # 存在するプラグインのみを追加
    for plugin_path in qt6_plugin_paths:
        if os.path.exists(plugin_path):
            for plugin_type in ['platforms', 'imageformats', 'styles']:
                full_path = os.path.join(plugin_path, plugin_type)
                if os.path.exists(full_path):
                    for file in os.listdir(full_path):
                        if file.endswith('.dll'):
                            src = os.path.join(full_path, file)
                            dst = os.path.join(plugin_type, file)
                            qt_plugins.append((dst, src, 'DATA'))
    
    # PyQt6関連のDLLも収集
    qt6_core_path = os.path.join(pyqt6_path, 'Qt6', 'bin')
    if os.path.exists(qt6_core_path):
        for file in os.listdir(qt6_core_path):
            if file.endswith('.dll'):
                src = os.path.join(qt6_core_path, file)
                qt_plugins.append((file, src, 'DATA'))
    
    # PyQt6モジュール内のDLLも含める
    for file in os.listdir(pyqt6_path):
        if file.endswith('.pyd'):
            src = os.path.join(pyqt6_path, file)
            qt_plugins.append((os.path.join('PyQt6', file), src, 'DATA'))
except ImportError:
    print("PyQt6がインストールされていません")

# 収集したプラグインを追加
a.datas += qt_plugins

# Qt設定ファイルを作成
with open('qt.conf', 'w') as f:
    f.write("[Paths]\nPlugins = .\n")

# qt.confファイルを追加
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