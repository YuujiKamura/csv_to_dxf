"""
processing.py
────────────────────────────────────────────
* CSV → DataFrame 抽出
* 単延長 ⇆ 累積距離 変換
* 測点名補完（既存測点を尊重）
────────────────────────────────────────────
"""
from __future__ import annotations

import io
import re
from pathlib import Path
from typing import Optional

import numpy as np
import pandas as pd

from src.station_name_utils import fill_station_names

# -----------------------------------------------------------
# 定数
# -----------------------------------------------------------
PITCH_M   = 20.0                     # 主測点間隔 [m]
ROUND_N   = 2                        # 数値丸め桁数
_REVERSE_LR: set[int] = set()        # 幅員入替え区間（今回は使わない）

# -----------------------------------------------------------
# 1. CSV 抽出
# -----------------------------------------------------------
_SECTION_RE = re.compile(r"^(区間\d+),台形計算", re.MULTILINE)


def extract_section_data(csv_path: Path | str, section_name: str) -> Optional[pd.DataFrame]:
    """
    区間ブロック（'区間X,台形計算'）を DataFrame として取得。
    ブロックが無ければファイル全体を単純 CSV とみなす。
    """
    try:
        csv_path = Path(csv_path)
        text     = csv_path.read_text(encoding='utf-8')

        m = re.search(rf"{section_name},台形計算[^\n]*\n(.+?)(?:\n区間\d+|$)", text, re.S)

        # ブロック抽出
        if m:
            block  = m.group(1).strip()
            raw_df = pd.read_csv(io.StringIO(block), comment='#')
        # 単純 CSV
        else:
            raw_df = pd.read_csv(csv_path, comment='#')

        # 列名マッピング
        mapping = {'測点名': 'name', '単延長L': 'x', '幅員W': 'wl'}
        raw_df  = raw_df.rename(columns={k: v for k, v in mapping.items() if k in raw_df.columns})

        # wr 列補完
        if 'wr' not in raw_df.columns:
            raw_df['wr'] = 0.0

        # 必須列を保証
        required = {'name': str, 'x': float, 'wl': float, 'wr': float}
        for col, typ in required.items():
            if col not in raw_df.columns:
                raw_df[col] = pd.NA
        df = raw_df[list(required.keys())].astype({**required, 'name': str}, errors='ignore')

        # 測点名 NaN → 空文字
        df['name'] = df['name'].fillna('').astype(str)

        # x が NaN の行を除外
        df = df.dropna(subset=['x'])
        return df

    except Exception as e:
        print(f"[ERROR] extract_section_data: {e}")
        return None


# -----------------------------------------------------------
# 2. 距離判定・変換
# -----------------------------------------------------------
def _is_cumulative(values: np.ndarray, pitch: float = PITCH_M) -> bool:
    """
    配列が累積距離かどうか判定  
    - 単調増加  
    - 増分の中央値 < 0.8 × pitch (≒ 16 m) なら累積とみなす
    """
    if values.size < 4:
        return False

    diffs = np.diff(values)
    if np.any(diffs <= 0):
        return False

    return np.median(diffs) < pitch * 0.8


def to_cumulative(df: pd.DataFrame, dist_col: str = 'x') -> pd.DataFrame:
    """
    距離列を累積距離に統一。既に累積ならそのまま、単延長なら累積和 (cumsum)。
    """
    out  = df.copy()
    vals = out[dist_col].to_numpy(dtype=float)

    if not _is_cumulative(vals):
        out[dist_col] = out[dist_col].cumsum()

    return out


# -----------------------------------------------------------
# 3. 変換パイプライン
# -----------------------------------------------------------
def transform_section(df: pd.DataFrame, section_no: int) -> pd.DataFrame:
    """
    * 距離を累積化
    * 既存測点を維持しつつ不足分を補完
    * 数値丸め
    """
    df = to_cumulative(df)
    df = fill_station_names(df)        # 既存測点を保持
    # wl / wr 入替えが必要ならここで
    return df.round(ROUND_N)


# -----------------------------------------------------------
# 4. 利用可能区間の検出（← ここが欠けていた）
# -----------------------------------------------------------
def get_available_sections(csv_path: Path | str) -> list[str]:
    """
    ファイルに含まれる区間名一覧を返す。
    ① ファイル名 '区間X.csv' → それを返す  
    ② 本文に '区間X,台形計算' → すべて列挙  
    ③ どれも無ければ ['区間1']
    """
    try:
        csv_path = Path(csv_path)

        # ① ファイル名から
        m = re.match(r'(区間\d+)\.csv', csv_path.name)
        if m:
            return [m.group(1)]

        # ② 本文検索
        text = csv_path.read_text(encoding='utf-8')
        sections = _SECTION_RE.findall(text)
        if sections:
            return sections

        # ③ CSV が読めれば単一区間
        pd.read_csv(csv_path, nrows=1)
        return ['区間1']

    except Exception as e:
        print(f"[ERROR] get_available_sections: {e}")
        return []
