#!/usr/bin/env python
# -*- coding: utf-8 -*-

# PyQt6のDLLとプラグインパスを設定するヘルパー関数
import os
import sys

def setup_qt_paths():
    """PyQt6のDLLとプラグインパスを設定する"""
    # 実行ファイルの場所を取得
    if getattr(sys, 'frozen', False):
        # PyInstallerでパッケージ化されている場合
        base_path = os.path.dirname(sys.executable)
    else:
        # 通常のPythonスクリプトとして実行されている場合
        base_path = os.path.dirname(os.path.abspath(__file__))
    
    # Qt6用の環境変数を設定
    os.environ['QT_PLUGIN_PATH'] = base_path
    
    # Windowsの場合、DLLサーチパスを追加
    if sys.platform.startswith('win'):
        try:
            if hasattr(os, 'add_dll_directory'):
                paths_to_add = [
                    base_path,
                    os.path.join(base_path, 'platforms'),
                    os.path.join(base_path, 'imageformats'),
                    os.path.join(base_path, 'styles'),
                ]
                
                for path in paths_to_add:
                    if os.path.exists(path):
                        os.add_dll_directory(path)
        except Exception as e:
            print(f"DLLパスの追加に失敗: {e}")

# 最初にQtパスを設定
setup_qt_paths()

# --- DEBUG >>> ---------------------------------
import importlib, src
import src.processing as p
print("◆ processing が読み込まれたパス:", p.__file__)
print("◆ get_available_sections がある？", hasattr(p, "get_available_sections"))
print("◆ モジュール内の関数一覧:",
      [k for k, v in p.__dict__.items() if callable(v)])
# --- DEBUG <<< ---------------------------------

from PyQt6.QtWidgets import QApplication
from PyQt6.QtGui import QIcon

# 相対インポートのためのパス設定
sys.path.append(os.path.abspath(os.path.dirname(__file__)))
from gui.app_controller import AppController
from gui.ui.main_window import MainWindow

def main():
    """アプリケーションのエントリーポイント"""
    app = QApplication(sys.argv)
    app.setApplicationName("CSV to DXF Converter")
    
    # スタイルシートの設定
    app.setStyle("Fusion")
    
    # アプリケーションコントローラーの作成
    controller = AppController()
    
    # メインウィンドウの作成と表示
    window = MainWindow(controller)
    window.show()
    
    # アプリケーションの実行
    sys.exit(app.exec())

if __name__ == "__main__":
    main() 