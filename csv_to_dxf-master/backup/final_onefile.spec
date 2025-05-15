# -*- mode: python ; coding: utf-8 -*-

block_cipher = None

a = Analysis(
    ['main_patched.py'],
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

# PyQt6プラグインを明示的に追加
import os
import sys
import shutil
from PyInstaller.utils.hooks import collect_system_data_files
from PyInstaller.utils.hooks import collect_data_files

# 明示的にPyQt6のプラグインのみを収集
qt_plugins = []

try:
    import PyQt6
    pyqt6_path = os.path.dirname(PyQt6.__file__)
    print(f"PyQt6パス: {pyqt6_path}")
    
    # プラグインの明示的なパス
    qt6_bin_path = os.path.join(pyqt6_path, 'Qt6', 'bin')
    qt6_plugin_path = os.path.join(pyqt6_path, 'Qt6', 'plugins')
    
    # 必要なDLLファイルをコピー
    if os.path.exists(qt6_bin_path):
        for dll_file in os.listdir(qt6_bin_path):
            if dll_file.endswith('.dll'):
                src = os.path.join(qt6_bin_path, dll_file)
                dst = os.path.join('build', dll_file)
                try:
                    shutil.copy2(src, dst)
                    print(f"コピー: {src} -> {dst}")
                except Exception as e:
                    print(f"コピー失敗: {src}: {e}")
    
    # プラグインファイルをコピー
    plugin_types = ['platforms', 'imageformats', 'styles']
    for plugin_type in plugin_types:
        plugin_dir = os.path.join(qt6_plugin_path, plugin_type)
        if os.path.exists(plugin_dir):
            for plugin_file in os.listdir(plugin_dir):
                if plugin_file.endswith('.dll'):
                    src = os.path.join(plugin_dir, plugin_file)
                    dst = os.path.join('build', plugin_type, plugin_file)
                    try:
                        shutil.copy2(src, dst)
                        print(f"コピー: {src} -> {dst}")
                        # datasリストにも追加
                        qt_plugins.append((os.path.join(plugin_type, plugin_file), src, 'DATA'))
                    except Exception as e:
                        print(f"コピー失敗: {src}: {e}")
    
    # PyQt6モジュール内のDLLも含める
    for file in os.listdir(pyqt6_path):
        if file.endswith('.pyd'):
            src = os.path.join(pyqt6_path, file)
            dst = os.path.join('PyQt6', file)
            qt_plugins.append((dst, src, 'DATA'))
    
    # 必要なQt6 DLLを追加
    qt_core_dlls = ['Qt6Core.dll', 'Qt6Gui.dll', 'Qt6Widgets.dll', 'Qt6Svg.dll', 'Qt6Network.dll']
    for dll in qt_core_dlls:
        src = os.path.join(qt6_bin_path, dll)
        if os.path.exists(src):
            qt_plugins.append((dll, src, 'DATA'))
except ImportError:
    print("PyQt6がインストールされていません")

# 収集したプラグインを追加
a.datas += qt_plugins

# Qt設定ファイルを作成
with open('qt.conf', 'w') as f:
    f.write("[Paths]\nPlugins = .\n")
a.datas += [('qt.conf', 'qt.conf', 'DATA')]

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.zipfiles,
    a.datas,
    [],
    name='csv_to_dxf',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    onefile=True,
) 