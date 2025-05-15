import os
import sys
import tempfile
import time
import pytest
import ezdxf
from pathlib import Path
from PyQt6.QtWidgets import QApplication, QMessageBox, QFileDialog
from PyQt6.QtTest import QTest
from PyQt6.QtCore import Qt, QTimer

# ヘッドレスモード用の環境変数設定
os.environ["QT_QPA_PLATFORM"] = "offscreen"

# テスト用のパスを追加
sys.path.append(os.path.join(os.path.dirname(__file__), '..'))

from gui.ui.main_window import MainWindow
from gui.app_controller import AppController
import pandas as pd

# ezdxfとsrcのコア機能を直接インポート（GUIに依存しない低レベルテスト）
from src.exporter import export_dxf
import src.dxf_draw_tenkaiz as draw

@pytest.fixture(scope="module")
def app():
    # ハイDPI設定を無効化
    try:
        QApplication.setAttribute(Qt.ApplicationAttribute.AA_DisableHighDpiScaling)
    except AttributeError:
        try:
            QApplication.setAttribute(Qt.AA_DisableHighDpiScaling)
        except:
            pass
    
    app = QApplication.instance() or QApplication(sys.argv)
    yield app

@pytest.fixture
def main_window(app):
    controller = AppController()
    window = MainWindow(controller)
    yield window
    window.close()

def test_dxf_text_size_is_correctly_set_direct_test():
    """生成されたDXFファイルにテキストサイズが正しく設定されるかの直接テスト"""
    # テスト用の簡単なDataFrameを作成
    data = {
        'name': ['No.0', 'No.1', 'No.2', 'No.3'],
        'x': [0, 20, 40, 60],
        'wl': [1.5, 1.75, 2.0, 1.9],
        'wr': [1.5, 1.75, 2.0, 1.9]
    }
    df = pd.DataFrame(data)
    
    # カスタムテキストサイズを設定
    custom_text_size = 555.0  # 特徴的な値にして検索しやすくする
    
    # 一時ファイルを作成
    with tempfile.TemporaryDirectory() as temp_dir:
        output_file = os.path.join(temp_dir, "test_output.dxf")
        
        # DXF出力関数を直接呼び出し、カスタムテキストサイズを渡す
        draw_func = lambda msp, data: draw.draw_road_sections(msp, data, text_height=custom_text_size)
        export_dxf(df, output_file, draw_func)
        
        # ファイルが生成されたことを確認
        assert os.path.exists(output_file), "DXFファイルが生成されていません"
        
        # DXFファイルを読み込む
        doc = ezdxf.readfile(output_file)
        msp = doc.modelspace()
        
        # テキストエンティティを探す
        text_entities = msp.query('TEXT')
        
        # テキストエンティティが存在することを確認
        assert len(text_entities) > 0, "DXFファイルにテキストエンティティが見つかりません"
        
        # 各テキストエンティティの高さを確認
        text_heights = []
        for text in text_entities:
            # テキストの高さ属性を取得
            height = text.dxf.height
            text_heights.append((text.dxf.text, height))
        
        # 結果をコンソールに出力
        print(f"見つかったテキストエンティティ（全{len(text_entities)}個）:")
        for text, height in text_heights:
            print(f"テキスト: '{text}', 高さ: {height}")
        
        # 少なくとも1つのテキストが指定した高さであることを確認
        correct_height_entities = [
            (text, height) for text, height in text_heights 
            if abs(height - custom_text_size) < 0.001
        ]
        
        assert len(correct_height_entities) > 0, \
            f"テキストサイズが{custom_text_size}のテキストが見つかりません"
        
        if len(correct_height_entities) > 0:
            print(f"正しいテキストサイズ({custom_text_size})が設定されているテキスト:")
            for text, height in correct_height_entities:
                print(f"テキスト: '{text}', 高さ: {height}")
        
        print("テキストサイズが正しく設定されていることを確認しました") 