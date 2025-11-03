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

    # ───────────────────────────────
    # CSV パスをセット
    # ───────────────────────────────
    def read_csv_file(self, file_path: str | Path) -> bool:
        """ファイルパスを保持する（重い読み込みは後で実行）"""
        try:
            self.csv_path = Path(file_path).expanduser().resolve()
            return True
        except Exception as e:  # pragma: no cover
            print(f"[ERROR] CSV ファイルを開けませんでした: {e}")
            self.csv_path = None
            return False

    # ───────────────────────────────
    # 区間一覧を取得
    # ───────────────────────────────
    def get_available_sections(self) -> list[str]:
        if not self.csv_path:
            return []
        return get_available_sections(self.csv_path)

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
        """
        指定区間を抽出して整形する。

        Parameters
        ----------
        section_name : str
            「区間1」「区間2」など
        auto_convert : bool
            単延長 → 累積距離へ変換するか
        normalize_station_names : bool
            測点名を 0+10 形式で補正するか
        swap_sides : bool
            wl / wr を入れ替える
        width_assign : {'wl', 'wr', None}
            2 本を合計して片側に持たせる

        Returns
        -------
        Optional[pandas.DataFrame]
            整形後のデータフレーム（失敗時 None）
        """
        if not self.csv_path:
            return None

        if section_name in self.section_cache:
            return self.section_cache[section_name]

        # 1) 抽出
        raw = extract_section_data(self.csv_path, section_name)
        if raw is None or raw.empty:
            return None

        # 2) 基本変換（transform_section は位置引数のみ受付）
        sec_num = int(section_name[-1]) if section_name[-1].isdigit() else 1
        df = transform_section(raw, sec_num)

        # 3) 追加処理
        if normalize_station_names:
            df = fill_station_names(df)

        if auto_convert:
            df = to_cumulative(df)

        if swap_sides and {"wl", "wr"}.issubset(df.columns):
            df[["wl", "wr"]] = df[["wr", "wl"]]

        if width_assign in {"wl", "wr"} and {"wl", "wr"}.issubset(df.columns):
            df[width_assign] = df["wl"] + df["wr"]
            # もう片側はゼロに
            other = "wr" if width_assign == "wl" else "wl"
            df[other] = 0.0

        # キャッシュ & 現在区間
        self.section_cache[section_name] = df
        self.current_section = section_name
        return df

    # ───────────────────────────────
    # キャッシュ優先で取得
    # ───────────────────────────────
    def get_section(self, name: str) -> Optional[pd.DataFrame]:
        """キャッシュされた区間データを取得。なければ処理して取得"""
        # キャッシュを確認
        if name in self.section_cache:
            return self.section_cache[name]
        # なければ処理を実行
        return self.process_section_data(name)

    # GUI との互換用シノニム
    get_section_data = get_section

    # ───────────────────────────────
    # CSV / DXF 保存
    # ───────────────────────────────
    def save_csv(self, section_name: str, out_dir: str | Path) -> bool:
        df = self.get_section(section_name)
        if df is None:
            return False
        try:
            export_csv(df, Path(out_dir).expanduser().resolve() / f"{section_name}.csv")
            return True
        except Exception as e:  # pragma: no cover
            print(f"[ERROR] CSV 保存に失敗: {e}")
            return False

    def save_dxf(self, section_name: str, out_dir: str | Path, out_name: str = None) -> bool:
        """
        区間データをDXFファイルとして保存
        
        Args:
            section_name: 区間名
            out_dir: 出力ディレクトリ
            out_name: 出力ファイル名（指定なしの場合は区間名.dxf）
            
        Returns:
            bool: 成功したかどうか
        """
        # データを取得
        df = self.get_section(section_name)
        if df is None or df.empty:
            print(f"[ERROR] 区間 '{section_name}' のデータが見つかりません")
            return False
        
        try:
            # 出力パスの調整
            output_dir = Path(out_dir).expanduser().resolve()
            # out_nameが指定されていない場合はセクション名を使用
            filename = out_name or f"{section_name}.dxf"
            output_path = output_dir / filename
            
            print(f"[INFO] DXF出力: {output_path}")
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
