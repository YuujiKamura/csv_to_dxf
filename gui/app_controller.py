# core/app_controller.py
"""
アプリケーション全体のコントローラー。

* ファイルダイアログは使わず、UI 側から渡されたパスのみで処理
* スクリーンショットは ./screenshots/ に自動保存
* 未処理例外は ./error.log に記録
"""

from __future__ import annotations

import logging
import os
import sys
from datetime import datetime
from pathlib import Path
from typing import Optional

import pandas as pd
from PyQt6.QtGui import QScreen
from PyQt6.QtWidgets import QApplication

# ※ ディレクトリ構成に合わせて調整してください
from gui.core.csv_processor import CSVProcessor
from gui.core.dxf_generator import DXFGenerator
from gui.core.config_manager import ConfigManager


# ──────────────────────────────
# 例外を丸ごとログに残す
# ──────────────────────────────
logging.basicConfig(
    filename="error.log",
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(message)s",
)


def _excepthook(exc_type, exc_value, tb):
    logging.exception("Unhandled exception", exc_info=(exc_type, exc_value, tb))
    sys.__excepthook__(exc_type, exc_value, tb)


sys.excepthook = _excepthook


# ──────────────────────────────
# AppController
# ──────────────────────────────
class AppController:
    """GUI から呼ばれるアプリ全体のコントローラー"""

    # ---------------------------
    # 初期化
    # ---------------------------
    def __init__(self) -> None:
        self.csv_processor = CSVProcessor()
        self.dxf_generator = DXFGenerator()
        self.config = ConfigManager()

        self.current_file: Optional[str] = None        # 読み込んだ CSV ファイル
        self.current_section: Optional[str] = None     # 最後に処理した区間名
        self.processed_data: Optional[pd.DataFrame] = None

        # 出力ディレクトリ（設定値が無ければカレント）
        self.output_dir = Path(self.config.get("output_directory", os.getcwd())).resolve()

    # ---------------------------
    # ファイル読み込み
    # ---------------------------
    def load_file(self, file_path: str | os.PathLike) -> bool:
        path = Path(file_path).expanduser().resolve()
        if not path.is_file():
            logging.warning("File not found: %s", path)
            return False

        self.current_file = str(path)
        self.csv_processor = CSVProcessor(str(path))   # パスを渡して再生成

        self.config.set("last_directory", str(path.parent))
        self._add_recent_file(str(path))

        logging.info("Loaded file: %s", path)
        return True

    # ---------------------------
    # 区間データ処理
    # ---------------------------
    def process_section(
        self,
        section_name: str,
        *,
        auto_convert: bool = True,
        normalize_station_names: bool = False,
        swap_sides: bool = False,
        width_assign: str = "wl",
    ) -> Optional[pd.DataFrame]:

        if not self.current_file:
            logging.error("No file loaded")
            return None

        df = self.csv_processor.process_section_data(
            section_name,
            auto_convert=auto_convert,
            normalize_station_names=normalize_station_names,
            swap_sides=swap_sides,
            width_assign=width_assign,
        )

        if df is None or df.empty:
            logging.warning("Section '%s' is empty", section_name)
            return None

        self.processed_data = df
        self.current_section = section_name                 # ★ ここで保持
        logging.info("Processed section: %s", section_name)
        return df

    # ---------------------------
    # CSV 出力
    # ---------------------------
    def save_csv(self, filename: str) -> bool:
        if not self._has_data():
            return False

        out = self.output_dir / filename
        try:
            self.processed_data.to_csv(out, index=False, float_format="%.2f")
            logging.info("Saved CSV: %s", out)
            return True
        except Exception:
            logging.exception("CSV save failed")
            return False

    # ---------------------------
    # DXF 出力
    # ---------------------------
    def save_dxf(self, filename: str) -> bool:
        if not self._has_data():
            return False
        if not self.current_section:
            logging.error("Current section not set")
            return False

        out = self.output_dir / filename

        try:
            logging.debug("➡ save_dxf: section=%s, out=%s", self.current_section, out)   # ★追加①
            ok = self.csv_processor.save_dxf(self.current_section, out.parent, out.name)
            if not ok:
                logging.error("CSVProcessor.save_dxf() returned False")                  # ★追加②
            else:
                logging.info("Saved DXF: %s", out)
            return ok
        except Exception:
            logging.exception("DXF save failed")                                         # ★既存
            return False

    # ---------------------------
    # スクリーンショット
    # ---------------------------
    def save_screenshot(self) -> Optional[Path]:
        shot_dir = Path("screenshots")
        shot_dir.mkdir(exist_ok=True)
        ts = datetime.now().strftime("%Y%m%d_%H%M%S")
        out_path = shot_dir / f"screenshot_{ts}.png"

        try:
            screen = QApplication.primaryScreen()
            if screen is None:
                raise RuntimeError("primaryScreen() returned None")

            pix = screen.grabWindow(0)
            if pix.save(str(out_path)):
                pix.save(str(shot_dir / "app_preview.png"))  # 最新リンク
                logging.info("Screenshot saved: %s", out_path)
                return out_path
            raise IOError("pix.save() returned False")
        except Exception:
            logging.exception("Screenshot failed")
            return None

    # ---------------------------
    # GUI から使う API
    # ---------------------------
    def get_current_file_path(self) -> str:
        return self.current_file or ""

    def get_available_sections(self) -> list[str]:
        return self.csv_processor.get_available_sections()

    def has_processed_data(self) -> bool:
        return self._has_data()

    def get_recent_files(self) -> list[str]:
        return self.config.get("recent_files", [])

    def get_last_directory(self) -> str:
        return self.config.get("last_directory", str(self.output_dir))

    def get_output_directory(self) -> str:
        return str(self.output_dir)

    # ---------------------------
    # GUI 用ラッパー
    # ---------------------------
    def convert_to_dxf(self, output_path: str | os.PathLike) -> bool:
        path = Path(output_path).expanduser().resolve()
        return self.save_dxf(path.name)   # ファイル名だけ渡す

    def save_as_csv(self, output_path: str | os.PathLike) -> bool:
        path = Path(output_path).expanduser().resolve()
        return self.save_csv(path.name)

    # ---------------------------
    # 内部ユーティリティ
    # ---------------------------
    def _add_recent_file(self, file_path: str) -> None:
        files = self.get_recent_files()
        if file_path in files:
            files.remove(file_path)
        files.insert(0, file_path)
        self.config.set("recent_files", files[:10])

    def _has_data(self) -> bool:
        return self.processed_data is not None and not self.processed_data.empty
