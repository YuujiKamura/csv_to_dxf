#!/usr/bin/env python
# -*- coding: utf-8 -*-

# Qt6のインポートエラーを修正するヘルパーを先に実行
import qt6_helper
qt6_helper.fix_qt_import_error()

# --- DEBUG >>> ---------------------------------
import importlib, src
import src.processing as p
print("◆ processing が読み込まれたパス:", p.__file__)
print("◆ get_available_sections がある？", hasattr(p, "get_available_sections"))
print("◆ モジュール内の関数一覧:",
      [k for k, v in p.__dict__.items() if callable(v)])
# --- DEBUG <<< ---------------------------------

import sys
import os
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