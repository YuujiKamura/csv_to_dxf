#!/usr/bin/env python
# -*- coding: utf-8 -*-

import os
import sys

# DLLの検索パスを設定する関数
def setup_qt_dll_paths():
    if getattr(sys, 'frozen', False):
        # PyInstallerでバンドルされている場合
        base_path = os.path.dirname(sys.executable)
    else:
        base_path = os.path.dirname(os.path.abspath(__file__))
    
    # Qt6のプラグインパスを設定
    os.environ['QT_PLUGIN_PATH'] = os.path.join(base_path, 'plugins')
    
    # Windows環境でDLLパスを追加
    if sys.platform.startswith('win'):
        try:
            if hasattr(os, 'add_dll_directory'):
                for path in [
                    base_path,
                    os.path.join(base_path, 'PyQt6'),
                    os.path.join(base_path, 'PyQt6', 'Qt6', 'bin')
                ]:
                    if os.path.exists(path):
                        os.add_dll_directory(path)
        except Exception as e:
            print(f"DLLパスの追加に失敗: {e}")

# 先にDLLパスを設定
setup_qt_dll_paths()

# PyQt6のインポート
from PyQt6.QtWidgets import QApplication, QMainWindow, QLabel, QPushButton, QVBoxLayout, QWidget
from PyQt6.QtCore import Qt

class SimpleWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("PyQt6 テストアプリ")
        self.setGeometry(100, 100, 400, 200)
        
        # ウィジェットの作成
        self.central_widget = QWidget()
        self.setCentralWidget(self.central_widget)
        
        # レイアウトの設定
        layout = QVBoxLayout()
        self.central_widget.setLayout(layout)
        
        # ラベルの追加
        self.label = QLabel("PyQt6のテストアプリケーション")
        self.label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        layout.addWidget(self.label)
        
        # ボタンの追加
        self.button = QPushButton("クリック")
        self.button.clicked.connect(self.on_button_clicked)
        layout.addWidget(self.button)
    
    def on_button_clicked(self):
        self.label.setText("ボタンがクリックされました！")

def main():
    app = QApplication(sys.argv)
    window = SimpleWindow()
    window.show()
    sys.exit(app.exec())

if __name__ == "__main__":
    main() 