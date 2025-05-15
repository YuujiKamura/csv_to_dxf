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
        normalize_station_names: bool = True,
        width_assign: str | None = None,
    ) -> Optional[pd.DataFrame]:
        """区間データを取得して処理する

        Args:
            section_name: 区間名
            auto_convert: 単延長→累積距離変換の有無
            normalize_station_names: 測点名の正規化有無
            width_assign: wl または wr に全幅を割当てる場合はその値

        Returns:
            加工済みの区間データを含むDataFrame、またはNone
        """
        if not section_name:
            return None

        try:
            df = self.csv_processor.process_section_data(
                section_name,
                auto_convert=auto_convert,
                normalize_station_names=normalize_station_names,
                width_assign=width_assign,
            )
            # 取得できた場合、参照をキャッシュ
            self.processed_data = df
            return df
        except Exception as e:
            print(f"[ERROR] process_section_data: {e}")
            return None

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
    def convert_to_dxf(self, file_path: str, text_height: float = 350.0) -> bool:
        """処理済みデータをDXFとして保存

        Args:
            file_path: 保存先パス
            text_height: DXFテキストのサイズ

        Returns:
            bool: 成功したかどうか
        """
        if self.processed_data is None or self.processed_data.empty:
            return False

        try:
            # 保存時にテキストサイズを指定
            import src.dxf_draw_tenkaiz as draw
            from src.exporter import export_dxf

            # ラムダ関数で固定のtext_heightを渡す
            draw_func = lambda msp, df: draw.draw_road_sections(msp, df, text_height=text_height)
            export_dxf(self.processed_data, file_path, draw_func)
            
            # 成功時、出力ディレクトリを記憶
            self.last_output_dir = os.path.dirname(file_path)
            return True
        except Exception as e:
            print(f"[ERROR] convert_to_dxf: {e}")
            return False

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
