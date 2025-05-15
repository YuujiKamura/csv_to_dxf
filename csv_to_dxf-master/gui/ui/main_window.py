# gui/main_window.py
# ==========================================================
import os
import sys
import datetime
from pathlib import Path

import pandas as pd
from PyQt6.QtCore import Qt, QAbstractTableModel, QModelIndex
from PyQt6.QtGui import QAction
from PyQt6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QPushButton, QLabel, QComboBox, QFileDialog, QMessageBox,
    QMenu, QTabWidget, QGroupBox, QCheckBox, QProgressBar, QDialog,
    QRadioButton, QHeaderView, QAbstractItemView, QTableView, QDoubleSpinBox
)

# ----------------------------------------------------------
# アプリケーションロジック
# ----------------------------------------------------------
from gui.core.csv_processor import CSVProcessor

# ==========================================================
# 1. 固定セルサイズの TableView
# ==========================================================
class SectionTable(QTableView):
    """行高さ 28px、列幅固定の QTableView"""
    COLUMN_WIDTHS = [100, 80, 80, 80]  # name, x, wl, wr

    def __init__(self, parent=None):
        super().__init__(parent)

        # 行高さ固定
        self.verticalHeader().setDefaultSectionSize(28)

        # 列幅固定
        header = self.horizontalHeader()
        header.setSectionResizeMode(QHeaderView.ResizeMode.Fixed)
        for i, w in enumerate(self.COLUMN_WIDTHS):
            self.setColumnWidth(i, w)

        # オプション
        self.setSelectionBehavior(QAbstractItemView.SelectionBehavior.SelectRows)
        self.setSelectionMode(QAbstractItemView.SelectionMode.SingleSelection)
        self.setAlternatingRowColors(True)

# ==========================================================
# 2. Pandas → Qt モデル
# ==========================================================
class PandasTableModel(QAbstractTableModel):
    def __init__(self, data: pd.DataFrame):
        super().__init__()
        self._data = data

    def rowCount(self, parent=None):
        return self._data.shape[0]

    def columnCount(self, parent=None):
        return self._data.shape[1]

    def data(self, index: QModelIndex, role=Qt.ItemDataRole.DisplayRole):
        if not index.isValid():
            return None
        if role == Qt.ItemDataRole.DisplayRole:
            value = self._data.iloc[index.row(), index.column()]
            if isinstance(value, float):
                return f"{value:.2f}"
            return str(value)
        return None

    def headerData(self, section, orientation, role=Qt.ItemDataRole.DisplayRole):
        if role != Qt.ItemDataRole.DisplayRole:
            return None
        return (str(self._data.columns[section])
                if orientation == Qt.Orientation.Horizontal
                else str(section + 1))

# ==========================================================
# 3. 幅員割り当てダイアログ
# ==========================================================
class WidthAssignDialog(QDialog):
    def __init__(self, section_name, current=None):
        super().__init__()
        self.setWindowTitle("単幅員の割り当て")
        lay = QVBoxLayout(self)
        lay.addWidget(QLabel(f"{section_name} の単幅員Wをどちらに割り当てますか？"))
        self.left_radio = QRadioButton("左幅員 (wl)")
        self.right_radio = QRadioButton("右幅員 (wr)")
        (self.right_radio if current == "wr" else self.left_radio).setChecked(True)
        lay.addWidget(self.left_radio)
        lay.addWidget(self.right_radio)
        ok_btn = QPushButton("OK")
        ok_btn.clicked.connect(self.accept)
        lay.addWidget(ok_btn)

    def get_result(self) -> str:
        return "wl" if self.left_radio.isChecked() else "wr"

# ==========================================================
# 4. メインウィンドウ
# ==========================================================
class MainWindow(QMainWindow):
    def __init__(self, app_controller):
        super().__init__()
        self.app_controller = app_controller
        self.setWindowTitle("CSV to DXF Converter")
        self.setMinimumSize(1000, 700)

        # ---------- メインレイアウト ----------
        central = QWidget(self)
        self.setCentralWidget(central)
        self.main_layout = QVBoxLayout(central)

        # ---------- メニュー ----------
        self.create_menu_bar()

        # ---------- UI ----------
        self.create_ui_components()

        # ---------- 状態 ----------
        self.width_assignments: dict[str, str] = {}
        self.update_recent_files_menu()
        self.update_ui_state()

    # ------------------------------------------------------
    # メニュー
    # ------------------------------------------------------
    def create_menu_bar(self):
        file_menu = self.menuBar().addMenu("ファイル")

        act_open = QAction("CSVファイルを開く...", self)
        act_open.triggered.connect(self.browse_file)
        file_menu.addAction(act_open)

        self.recent_files_menu = QMenu("最近使用したファイル", self)
        file_menu.addMenu(self.recent_files_menu)

        act_shot = QAction("スクリーンショットを保存...", self)
        act_shot.triggered.connect(self.save_screenshot)
        file_menu.addAction(act_shot)

        file_menu.addSeparator()
        act_exit = QAction("終了", self)
        act_exit.triggered.connect(self.close)
        file_menu.addAction(act_exit)

    # ------------------------------------------------------
    # UI コンポーネント
    # ------------------------------------------------------
    def create_ui_components(self):
        # ---- ファイル選択 ----
        file_lay = QHBoxLayout()
        self.file_path_label = QLabel("CSVファイルを選択してください")
        browse_btn = QPushButton("参照...")
        browse_btn.clicked.connect(self.browse_file)
        file_lay.addWidget(QLabel("ファイル:"))
        file_lay.addWidget(self.file_path_label, 1)
        file_lay.addWidget(browse_btn)

        # ---- 区間選択 ----
        sec_lay = QHBoxLayout()
        self.section_combo = QComboBox()
        self.section_combo.setEnabled(False)
        self.section_combo.currentIndexChanged.connect(self.on_section_changed)
        ref_btn = QPushButton("区間更新")
        ref_btn.clicked.connect(self.refresh_sections)
        width_btn = QPushButton("幅員割り当て")
        width_btn.clicked.connect(self.assign_width_dialog)
        sec_lay.addWidget(QLabel("区間:"))
        sec_lay.addWidget(self.section_combo, 1)
        sec_lay.addWidget(ref_btn)
        sec_lay.addWidget(width_btn)

        # ---- 上部まとめ ----
        top_box = QVBoxLayout()
        top_box.addLayout(file_lay)
        top_box.addLayout(sec_lay)

        # ---- タブ ----
        self.tabs = QTabWidget()

        # データプレビュタブ
        self.data_tab = QWidget()
        data_lay = QVBoxLayout(self.data_tab)
        data_lay.addWidget(QLabel("データプレビュー:"))
        self.table_view = SectionTable()                        # ★ 置き換え
        data_lay.addWidget(self.table_view)

        # オプションタブ
        self.options_tab = QWidget()
        opt_lay = QVBoxLayout(self.options_tab)
        grp = QGroupBox("処理オプション")
        grp_lay = QVBoxLayout(grp)
        self.auto_convert_check = QCheckBox("単延長を自動的に累積距離に変換する")
        self.auto_convert_check.setChecked(True)
        self.normalize_names_check = QCheckBox("測点名を連番形式に正規化する")
        self.normalize_names_check.setChecked(True)
        grp_lay.addWidget(self.auto_convert_check)
        grp_lay.addWidget(self.normalize_names_check)
        opt_lay.addWidget(grp)
        
        # DXFテキストサイズの設定を追加
        grp_dxf = QGroupBox("DXF出力オプション")
        grp_dxf_lay = QVBoxLayout(grp_dxf)
        
        # テキストサイズの設定
        text_size_lay = QHBoxLayout()
        text_size_label = QLabel("テキストサイズ:")
        self.text_size_spin = QDoubleSpinBox()
        self.text_size_spin.setRange(100.0, 1000.0)
        self.text_size_spin.setValue(350.0)  # デフォルト値
        self.text_size_spin.setSingleStep(10.0)
        self.text_size_spin.setSuffix(" pt")
        text_size_lay.addWidget(text_size_label)
        text_size_lay.addWidget(self.text_size_spin)
        text_size_lay.addStretch()
        
        grp_dxf_lay.addLayout(text_size_lay)
        opt_lay.addWidget(grp_dxf)
        
        opt_lay.addStretch()

        self.tabs.addTab(self.data_tab, "データプレビュー")
        self.tabs.addTab(self.options_tab, "変換オプション")

        # ---- アクションボタン ----
        act_lay = QHBoxLayout()
        self.progress_bar = QProgressBar()
        self.progress_bar.setVisible(False)
        self.preview_button = QPushButton("プレビュー")
        self.preview_button.clicked.connect(self.preview_data)
        self.preview_button.setEnabled(False)
        self.save_csv_button = QPushButton("CSVとして保存")
        self.save_csv_button.clicked.connect(self.save_as_csv)
        self.save_csv_button.setEnabled(False)
        self.convert_button = QPushButton("DXFに変換")
        self.convert_button.clicked.connect(self.convert_to_dxf)
        self.convert_button.setEnabled(False)
        act_lay.addWidget(self.progress_bar)
        act_lay.addStretch()
        act_lay.addWidget(self.preview_button)
        act_lay.addWidget(self.save_csv_button)
        act_lay.addWidget(self.convert_button)

        # ---- メインレイアウト ----
        self.main_layout.addLayout(top_box)
        self.main_layout.addWidget(self.tabs, 1)
        self.main_layout.addLayout(act_lay)

    # ------------------------------------------------------
    # UI 状態更新
    # ------------------------------------------------------
    def update_ui_state(self):
        has_file = bool(self.app_controller.get_current_file_path())
        has_sections = len(self.app_controller.get_available_sections()) > 0
        has_data = self.app_controller.has_processed_data()

        self.section_combo.setEnabled(has_file and has_sections)
        self.preview_button.setEnabled(has_file and has_sections)
        self.save_csv_button.setEnabled(has_data)
        self.convert_button.setEnabled(has_data)

    # ------------------------------------------------------
    # 最近ファイルメニュー更新
    # ------------------------------------------------------
    def update_recent_files_menu(self):
        self.recent_files_menu.clear()
        recent_files = self.app_controller.get_recent_files()
        if not recent_files:
            dummy = QAction("最近使用したファイルはありません", self)
            dummy.setEnabled(False)
            self.recent_files_menu.addAction(dummy)
            return
        for path in recent_files:
            act = QAction(os.path.basename(path), self)
            act.setToolTip(path)
            act.triggered.connect(lambda _, p=path: self.open_recent_file(p))
            self.recent_files_menu.addAction(act)

    # ------------------------------------------------------
    # ファイル操作
    # ------------------------------------------------------
    def browse_file(self):
        file_path, _ = QFileDialog.getOpenFileName(
            self, "CSVファイルを選択",
            self.app_controller.get_last_directory(),
            "CSV Files (*.csv);;All Files (*)"
        )
        if file_path:
            self.open_file(file_path)

    def open_recent_file(self, path):
        self.open_file(path)

    def open_file(self, file_path):
        if not os.path.exists(file_path):
            QMessageBox.warning(self, "エラー", f"ファイル '{file_path}' が見つかりません。")
            return
        if self.app_controller.load_file(file_path):
            self.file_path_label.setText(os.path.basename(file_path))
            self.refresh_sections()
            self.update_recent_files_menu()
            self.update_ui_state()
            if self.section_combo.count() > 0:
                self.section_combo.setCurrentIndex(0)
                self.preview_data()
        else:
            QMessageBox.warning(self, "エラー", f"ファイル '{file_path}' を開けませんでした。")

    # ------------------------------------------------------
    # 区間選択・幅員割り当て
    # ------------------------------------------------------
    def refresh_sections(self):
        self.section_combo.clear()
        self.section_combo.addItems(self.app_controller.get_available_sections())

    def on_section_changed(self, index):
        if index >= 0:
            self.preview_data()

    def assign_width_dialog(self):
        section = self.section_combo.currentText()
        if not section:
            return
        dlg = WidthAssignDialog(section, self.width_assignments.get(section))
        if dlg.exec():
            self.width_assignments[section] = dlg.get_result()
            self.preview_data()
            # --- ここでCSV上書き保存 ---
            df = self.app_controller.processed_data
            csv_path = self.app_controller.get_current_file_path()
            if df is not None and csv_path:
                try:
                    df.to_csv(csv_path, index=False, encoding="utf-8", float_format="%.2f")
                except Exception as e:
                    from PyQt6.QtWidgets import QMessageBox
                    QMessageBox.warning(self, "保存エラー", f"CSV保存に失敗しました: {e}")

    # ------------------------------------------------------
    # プレビュー処理
    # ------------------------------------------------------
    def preview_data(self):
        sel_section = self.section_combo.currentText()
        if not sel_section:
            return

        self.progress_bar.setVisible(True)
        self.progress_bar.setValue(20)
        QApplication.processEvents()

        df = self.app_controller.process_section(
            sel_section,
            auto_convert=self.auto_convert_check.isChecked(),
            normalize_station_names=self.normalize_names_check.isChecked(),
            width_assign=self.width_assignments.get(sel_section, "wl")
        )

        self.progress_bar.setValue(80)
        QApplication.processEvents()

        if df is not None and not df.empty:
            self.table_view.setModel(PandasTableModel(df))
            self.update_ui_state()
            self.statusBar().showMessage(f"{len(df)} 行を表示")
        else:
            self.table_view.setModel(None)
            QMessageBox.warning(self, "警告", f"区間 '{sel_section}' のデータを処理できませんでした。")

        self.progress_bar.setValue(100)
        self.progress_bar.setVisible(False)

    # ------------------------------------------------------
    # 保存 & 変換
    # ------------------------------------------------------
    def save_as_csv(self):
        if not self.app_controller.has_processed_data():
            return
        section = self.section_combo.currentText()
        default = os.path.join(self.app_controller.get_output_directory(), f"{section}.csv")
        file_path, _ = QFileDialog.getSaveFileName(self, "CSVファイルを保存", default, "CSV Files (*.csv)")
        if file_path and self.app_controller.save_as_csv(file_path):
            QMessageBox.information(self, "成功", f"保存しました: {file_path}")

    def convert_to_dxf(self):
        if not self.app_controller.has_processed_data():
            return
        section = self.section_combo.currentText()
        default = os.path.join(self.app_controller.get_output_directory(), f"{section}.dxf")
        file_path, _ = QFileDialog.getSaveFileName(self, "DXFファイルを保存", default, "DXF Files (*.dxf)")
        
        if file_path:
            # テキストサイズを取得してDXF変換に渡す
            text_height = self.text_size_spin.value()
            if self.app_controller.convert_to_dxf(file_path, text_height=text_height):
                QMessageBox.information(self, "成功", f"生成しました: {file_path}")
            else:
                QMessageBox.warning(self, "エラー", "DXFファイルの生成に失敗しました。")

    # gui/ui/main_window.py  ─────────────────────────────────────────
    # 例：class MainWindow(QMainWindow): の中

    # ------------------------------------------------------
    # スクリーンショット（メニュー「スクリーンショットを保存」用）
    # ------------------------------------------------------
    def save_screenshot(self) -> None:
        """ウィンドウを PNG で保存し、完了ダイアログを表示する"""
        from datetime import datetime
        from pathlib import Path
        from PyQt6.QtWidgets import QMessageBox

        # プロジェクトルート/screenshots フォルダ
        scr_dir = Path(__file__).resolve().parents[2] / "screenshots"
        scr_dir.mkdir(exist_ok=True)

        # ファイル名（最新リンクと時刻付き）
        ts = datetime.now().strftime("%Y%m%d_%H%M%S")
        latest_path = scr_dir / "app_preview.png"
        ts_path = scr_dir / f"app_preview_{ts}.png"

        # 一度だけ grab して 2 枚書き出し
        pix = self.grab()
        ok1 = pix.save(str(ts_path))
        ok2 = pix.save(str(latest_path))

        if ok1 and ok2:
            QMessageBox.information(self, "完了",
                                    f"スクリーンショットを保存しました:\n{ts_path}")
        else:
            QMessageBox.warning(self, "失敗",
                                "スクリーンショットの保存に失敗しました")


# ==========================================================
# アプリ起動（スクリプト直実行時用）
# ==========================================================
if __name__ == "__main__":
    from gui.app_controller import AppController   # 適宜 import パス調整
    app = QApplication(sys.argv)
    win = MainWindow(AppController())
    win.show()
    sys.exit(app.exec())
