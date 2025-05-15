import os
import sys
import importlib.util

def fix_qt_import_error():
    """PyQt6/PySide6のインポートエラーを修正するヘルパー関数"""
    # 実行パスを取得
    if getattr(sys, 'frozen', False):
        # PyInstallerでパッケージ化されている場合
        application_path = os.path.dirname(sys.executable)
    else:
        # 通常のPythonスクリプトとして実行されている場合
        application_path = os.path.dirname(os.path.abspath(__file__))
    
    # Qt6のプラグインパスを設定
    os.environ['QT_PLUGIN_PATH'] = os.path.join(application_path, 'PyQt6', 'Qt6', 'plugins')
    
    # ライブラリパスも設定
    os.environ['QT_QPA_PLATFORM_PLUGIN_PATH'] = os.path.join(application_path, 'platforms')
    
    # Windows環境の場合、DLLサーチパスを追加
    if sys.platform.startswith('win'):
        try:
            # Python 3.8以降でサポート
            if hasattr(os, 'add_dll_directory'):
                # 可能性のあるDLLのパスを追加
                dll_paths = [
                    os.path.join(application_path),
                    os.path.join(application_path, 'PyQt6'),
                    os.path.join(application_path, 'PyQt6', 'Qt6', 'bin'),
                ]
                
                # 存在するパスのみを追加
                for path in dll_paths:
                    if os.path.exists(path):
                        os.add_dll_directory(path)
        except Exception as e:
            print(f"DLLディレクトリの追加に失敗: {e}")

if __name__ == "__main__":
    fix_qt_import_error() 