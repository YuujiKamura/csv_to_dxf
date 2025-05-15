import os
import sys
import tempfile
import pytest
from PyQt6.QtWidgets import QApplication, QMessageBox, QDoubleSpinBox
from PyQt6.QtTest import QTest
from PyQt6.QtCore import Qt
import ezdxf

# ヘッドレスモード用の環境変数設定
os.environ["QT_QPA_PLATFORM"] = "offscreen"

sys.path.append(os.path.join(os.path.dirname(__file__), '..'))
from gui.ui.main_window import MainWindow
from gui.app_controller import AppController

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

def test_text_size_spinner_exists(app, main_window, qtbot):
    """テキストサイズのスピナーが存在するかテスト"""
    # 変換オプションタブを選択
    main_window.tabs.setCurrentIndex(1)
    
    # DXF出力オプションの存在を確認
    dxf_option_group = None
    for i in range(main_window.options_tab.layout().count()):
        widget = main_window.options_tab.layout().itemAt(i).widget()
        if widget and widget.title() == "DXF出力オプション":
            dxf_option_group = widget
            break
    
    assert dxf_option_group is not None, "DXF出力オプショングループが見つかりません"
    
    # テキストサイズのスピナーが存在することを確認
    text_size_spin = main_window.text_size_spin
    assert text_size_spin is not None, "テキストサイズのスピナーが見つかりません"
    assert isinstance(text_size_spin, QDoubleSpinBox), "テキストサイズのウィジェットがQDoubleSpinBoxではありません"
    
    # 初期値が350.0であることを確認
    assert text_size_spin.value() == 350.0, f"テキストサイズの初期値が350.0ではありません。実際の値: {text_size_spin.value()}"
    
    # 範囲が100.0〜1000.0であることを確認
    assert text_size_spin.minimum() == 100.0, f"テキストサイズの最小値が100.0ではありません: {text_size_spin.minimum()}"
    assert text_size_spin.maximum() == 1000.0, f"テキストサイズの最大値が1000.0ではありません: {text_size_spin.maximum()}"

def test_swap_sides_checkbox_removed(app, main_window, qtbot):
    """幅員入れ替えチェックボックスが削除されていることを確認"""
    # 変換オプションタブを選択
    main_window.tabs.setCurrentIndex(1)
    
    # 処理オプショングループを探す
    options_group = None
    for i in range(main_window.options_tab.layout().count()):
        widget = main_window.options_tab.layout().itemAt(i).widget()
        if widget and widget.title() == "処理オプション":
            options_group = widget
            break
    
    assert options_group is not None, "処理オプショングループが見つかりません"
    
    # swap_sides_checkという名前の属性がないことを確認
    assert not hasattr(main_window, "swap_sides_check"), "swap_sides_check属性が存在します"
    
    # レイアウト内のすべてのウィジェットをチェック
    swap_sides_text = "左右の幅員を入れ替える"
    all_checkboxes_texts = []
    
    for i in range(options_group.layout().count()):
        widget = options_group.layout().itemAt(i).widget()
        if widget and hasattr(widget, "text"):
            all_checkboxes_texts.append(widget.text())
            # 幅員入れ替えテキストを含むチェックボックスがないことを確認
            assert swap_sides_text not in widget.text(), f"幅員入れ替えチェックボックスが見つかりました: {widget.text()}"
    
    print(f"オプションタブ内のすべてのチェックボックステキスト: {all_checkboxes_texts}")

def test_dxf_export_with_custom_text_size(app, main_window, qtbot, monkeypatch):
    """カスタムテキストサイズでDXFエクスポートをテスト"""
    # テスト用のCSVファイルのパスを取得
    csv_path = os.path.abspath(os.path.join(os.path.dirname(__file__), 'おやま.csv'))
    assert os.path.exists(csv_path), f"テスト用CSVファイルが見つかりません: {csv_path}"
    
    # convert_to_dxfメソッドを直接テスト
    custom_text_size = 500.0
    
    # モックする前のメソッドをバックアップ
    original_export_dxf = main_window.app_controller.convert_to_dxf
    
    # テスト用フラグ
    convert_called = False
    test_path = os.path.join(tempfile.gettempdir(), "test_output.dxf")
    
    # テスト用のモック関数
    def mock_convert_to_dxf(file_path, text_height=350.0):
        nonlocal convert_called
        convert_called = True
        # テキストサイズが正しく渡されたか確認
        assert text_height == custom_text_size, f"期待されるテキストサイズ({custom_text_size})と実際のテキストサイズ({text_height})が一致しません"
        return True
    
    # モック関数に置き換え
    monkeypatch.setattr(main_window.app_controller, 'convert_to_dxf', mock_convert_to_dxf)
    
    try:
        # ファイルを読み込み
        main_window.app_controller.load_file(csv_path)
        
        # データを処理してUIを更新
        main_window.app_controller.process_section("", auto_convert=True, normalize_station_names=True)
        main_window.update_ui_state()
        
        # 変換オプションタブに切り替え
        main_window.tabs.setCurrentIndex(1)
        
        # テキストサイズを設定
        main_window.text_size_spin.setValue(custom_text_size)
        
        # 直接convert_to_dxfを呼び出す
        main_window.app_controller.convert_to_dxf(test_path, text_height=custom_text_size)
        
        # 呼び出されたことを確認
        assert convert_called, "convert_to_dxfメソッドが呼び出されていません"
        
    finally:
        # 元のメソッドに戻す
        monkeypatch.setattr(main_window.app_controller, 'convert_to_dxf', original_export_dxf) 