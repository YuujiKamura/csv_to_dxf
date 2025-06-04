# gui/core/csv_processor.py
"""
CSVProcessor  ―  GUI レイヤから呼び出される薄いファサード

機能
-----
1. CSV から区間データを抽出            … src.processing.extract_section_data
2. 必要な前処理・後処理を実施            … fill_station_names / to_cumulative など
3. DataFrame をキャッシュして再利用
4. CSV / DXF ファイルへのエクスポート   … src.exporter.export_csv / export_dxf
"""

from __future__ import annotations

from pathlib import Path
from typing import Dict, Optional

import pandas as pd

from src.processing import (
    extract_section_data,
    transform_section,
    get_available_sections,
    fill_station_names,
    to_cumulative,
)
from src.exporter import export_csv, export_dxf
import src.dxf_draw_tenkaiz as draw


class CSVProcessor:
    """区間データの抽出・整形・保存を担当するクラス"""

    # ───────────────────────────────
    # 初期化
    # ───────────────────────────────
    def __init__(self, csv_path: str | None = None) -> None:
        self.csv_path: Optional[Path] = Path(csv_path).expanduser().resolve() if csv_path else None
        self.section_cache: Dict[str, pd.DataFrame] = {}
        self.current_section: Optional[str] = None
        self._csv_data: Optional[pd.DataFrame] = None

    # ------------------------------------------------------------------
    # 新規API: file_path プロパティ (csv_path のエイリアス)
    # ------------------------------------------------------------------
    @property
    def file_path(self) -> Optional[str]:
        return str(self.csv_path) if self.csv_path else None

    @file_path.setter
    def file_path(self, value: str | Path | None) -> None:
        self.csv_path = Path(value).expanduser().resolve() if value else None

    # ------------------------------------------------------------------
    # 新規API: csv_data プロパティ
    # ------------------------------------------------------------------
    @property
    def csv_data(self) -> Optional[pd.DataFrame]:
        return self._csv_data

    # ───────────────────────────────
    # CSV パスをセット
    # ───────────────────────────────
    def read_csv_file(self, file_path: str | Path) -> bool:
        print(f"[LOG] read_csv_file: file_path={file_path}")
        try:
            path = Path(file_path).expanduser().resolve()
            if not path.is_file():
                print("[ERROR] CSV ファイルを開けませんでした: file not found")
                return False
            self.csv_path = path
            self._csv_data = pd.read_csv(self.csv_path)
            print(f"[LOG] read_csv_file: self.csv_path set to {self.csv_path}")
            return True
        except Exception as e:  # pragma: no cover
            print(f"[ERROR] CSV ファイルを開けませんでした: {e}")
            self.csv_path = None
            self._csv_data = None
            return False

    # ───────────────────────────────
    # 区間一覧を取得
    # ───────────────────────────────
    def get_available_sections(self) -> list[str]:
        print(f"[LOG] get_available_sections: csv_path={self.csv_path}")
        if not self.csv_path:
            print("[LOG] get_available_sections: no csv_path")
            return []
        sections = get_available_sections(self.csv_path)
        print(f"[LOG] get_available_sections: sections={sections}")
        return sections

    # ------------------------------------------------------------------
    # 新規API: 区間データの直接抽出
    # ------------------------------------------------------------------
    def extract_section_data(self, section_name: str) -> Optional[pd.DataFrame]:
        if not self.csv_path:
            return None
        try:
            df = extract_section_data(self.csv_path, section_name)
            return df
        except Exception as e:  # pragma: no cover
            print(f"[ERROR] extract_section_data failed: {e}")
            return None

    # ───────────────────────────────
    # 区間データを DataFrame 化
    # ───────────────────────────────
    def process_section_data(
        self,
        section_name: str,
        *,
        auto_convert: bool = True,
        normalize_station_names: bool = True,
        swap_sides: bool = False,
        width_assign: str | None = None,
    ) -> Optional[pd.DataFrame]:
        print(f"[LOG] process_section_data: section_name={section_name}, auto_convert={auto_convert}, normalize_station_names={normalize_station_names}, swap_sides={swap_sides}, width_assign={width_assign}")
        if not self.csv_path:
            print("[LOG] process_section_data: no csv_path")
            return None

        # width_assignが指定されている場合は、キャッシュを無視して割当処理を実行するために
        # キャッシュキーとして元の区間名だけでなく、幅員割当オプションも含める
        cache_key = f"{section_name}_{width_assign}" if width_assign else section_name
        
        if cache_key in self.section_cache:
            print(f"[LOG] process_section_data: cache hit for {cache_key}")
            return self.section_cache[cache_key]
        print(f"[LOG] process_section_data: cache miss for {cache_key}")

        # 元のデータをキャッシュから取得するか、なければ処理する
        if section_name in self.section_cache and width_assign:
            # 幅員割当だけが変更された場合、元のデータをコピーして使用
            print(f"[LOG] process_section_data: using cached base data for {section_name}")
            df = self.section_cache[section_name].copy()
        else:
            # 1) 抽出
            raw = self.extract_section_data(section_name)
            print(f"[LOG] process_section_data: raw shape={getattr(raw, 'shape', None)}")
            if raw is None or raw.empty:
                print(f"[LOG] process_section_data: raw is None or empty")
                return None

            # 2) 基本変換（transform_section は位置引数のみ受付）
            # 空文字列の場合にIndexErrorが発生しないように修正
            sec_num = 1
            if section_name and section_name[-1].isdigit():
                sec_num = int(section_name[-1])
            
            df = transform_section(raw, sec_num)
            print(f"[LOG] process_section_data: after transform_section shape={df.shape}")

            # 3) 追加処理
            if normalize_station_names:
                df = fill_station_names(df)
                print(f"[LOG] process_section_data: after fill_station_names shape={df.shape}")

            if auto_convert:
                df = to_cumulative(df)
                print(f"[LOG] process_section_data: after to_cumulative shape={df.shape}")

            # 基本的なデータはキャッシュしておく
            if width_assign:
                # 幅員割当前のデータもキャッシュ
                self.section_cache[section_name] = df.copy()
                print(f"[LOG] process_section_data: cached base data {section_name}, df shape={df.shape}")

        if swap_sides and {"wl", "wr"}.issubset(df.columns):
            df[["wl", "wr"]] = df[["wr", "wl"]]
            print(f"[LOG] process_section_data: after swap_sides shape={df.shape}")

        if width_assign in {"wl", "wr"} and {"wl", "wr"}.issubset(df.columns):
            df[width_assign] = df["wl"] + df["wr"]
            other = "wr" if width_assign == "wl" else "wl"
            df[other] = 0.0
            print(f"[LOG] process_section_data: after width_assign shape={df.shape}")

        # 最終的な結果をキャッシュ
        self.section_cache[cache_key] = df
        self.current_section = section_name
        print(f"[LOG] process_section_data: cached {cache_key}, df shape={df.shape}")
        return df

    # ───────────────────────────────
    # キャッシュ優先で取得
    # ───────────────────────────────
    def get_section(self, name: str) -> Optional[pd.DataFrame]:
        print(f"[LOG] get_section: name={name}")
        if name in self.section_cache:
            print(f"[LOG] get_section: cache hit for {name}")
            return self.section_cache[name]
        print(f"[LOG] get_section: cache miss for {name}")
        return self.process_section_data(name)

    # GUI との互換用シノニム
    get_section_data = get_section

    # ───────────────────────────────
    # CSV / DXF 保存
    # ───────────────────────────────
    def save_csv(self, section_name: str, out_dir: str | Path) -> bool:
        print(f"[LOG] save_csv: section_name={section_name}, out_dir={out_dir}")
        df = self.get_section(section_name)
        if df is None:
            print(f"[LOG] save_csv: df is None")
            return False
        try:
            out_path = Path(out_dir).expanduser().resolve() / f"{section_name}.csv"
            print(f"[LOG] save_csv: saving to {out_path}")
            export_csv(df, out_path)
            print(f"[LOG] save_csv: saved to {out_path}")
            return True
        except Exception as e:  # pragma: no cover
            print(f"[ERROR] CSV 保存に失敗: {e}")
            return False

    # ------------------------------------------------------------------
    # 新規API: 処理済みDataFrameを直接保存
    # ------------------------------------------------------------------
    def save_processed_data(self, df: pd.DataFrame, file_path: str | Path) -> bool:
        try:
            out = Path(file_path).expanduser().resolve()
            export_csv(df, out)
            return True
        except Exception as e:  # pragma: no cover
            print(f"[ERROR] save_processed_data failed: {e}")
            return False

    def save_dxf(self, section_name: str, out_dir: str | Path, out_name: str = None) -> bool:
        print(f"[LOG] save_dxf: section_name={section_name}, out_dir={out_dir}, out_name={out_name}")
        df = self.get_section(section_name)
        if df is None or df.empty:
            print(f"[LOG] save_dxf: df is None or empty")
            print(f"[ERROR] 区間 '{section_name}' のデータが見つかりません")
            return False
        try:
            output_dir = Path(out_dir).expanduser().resolve()
            filename = out_name or f"{section_name}.dxf"
            output_path = output_dir / filename
            print(f"[LOG] save_dxf: saving to {output_path}")
            export_dxf(
                df,
                output_path,
                draw.draw_road_sections,
            )
            print(f"[INFO] DXF保存成功: {output_path}")
            return True
        except Exception as e:  # pragma: no cover
            print(f"[ERROR] DXF 保存に失敗: {e}")
            return False
