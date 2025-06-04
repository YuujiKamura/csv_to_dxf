"""
station_name_utils.py
────────────────────────────────────────────
* 距離 ↔ 測点名 の相互変換
* 既存測点を活かしつつ不足分を補完
  ── 仕様 ──────────────────────────────
  1. 主測点 (sub=0) を補完した行のみ `No.` プレフィックスを付与
  2. CSV に既存測点があれば、以降の補完はその測点を“新しい原点”
  3. 互換 API:
       - fill_station_names()     … 新 API
       - complete_station_names() … 旧名ラッパー
       - preserve_and_complete_station_names() … 旧名ラッパー
────────────────────────────────────────────
"""
from __future__ import annotations

import re
from decimal import Decimal, ROUND_HALF_UP
from typing import Optional, Tuple

import pandas as pd

_STATION_RE   = re.compile(r'^(?:No\.)?(?P<main>\d+)(?:\+(?P<sub>[\d.]+))?$')
_SPAN         = 20                            # 主測点間隔 [m]
_Q            = Decimal('0.1')                # サブ距離丸め桁 (=0.1 m)


# ─────────────────────────
# ヘルパー
# ─────────────────────────
def _round(val, q=_Q):
    return Decimal(str(val)).quantize(q, ROUND_HALF_UP)


def _parse(name: str) -> Optional[Tuple[int, float]]:
    m = _STATION_RE.match(str(name))
    if not m:
        return None
    return int(m.group('main')), float(m.group('sub') or 0)


def _name_from_dist(dist: float) -> str:
    """
    累積距離 → 測点名  
    * 主測点 (sub==0) のときだけ `No.` プレフィックス
    """
    main = int(dist // _SPAN)
    sub  = _round(dist % _SPAN)

    if sub == 0:
        return f'No.{main}'

    sub_str = str(sub.normalize()) if sub % 1 else str(int(sub))
    return f'{main}+{sub_str}'


def _is_valid(name: Optional[str]) -> bool:
    return bool(_STATION_RE.match(str(name))) if pd.notna(name) else False


# ─────────────────────────
# メイン API
# ─────────────────────────
def fill_station_names(df: pd.DataFrame,
                       *,
                       name_col: str = 'name',
                       dist_col: str = 'x',
                       keep_existing: bool = True) -> pd.DataFrame:
    """
    距離列 `dist_col` から測点名を補完／再生成
      • 既存の正しい測点名は保持 (keep_existing=True)
      • 既存測点が現れた行を“新しい起点”として以後を補完
      • 主測点を補完した場合のみ `No.` を付与
    """
    if {name_col, dist_col} - set(df.columns):
        raise KeyError(f"'{name_col}' または '{dist_col}' が DataFrame にありません")

    out = df.copy()

    # x=0 かつ空欄 → No.0
    mask_x0 = (out[dist_col] == 0) & (out[name_col].isna() | (out[name_col] == ''))
    out.loc[mask_x0, name_col] = 'No.0'

    offset = 0.0                        # x → 累積距離 へ変換するためのずれ[m]
    names  = list(out[name_col])
    xs     = list(out[dist_col])

    original_valid = df[name_col].apply(_is_valid)

    for i in range(len(out)):
        name = names[i]
        x    = float(xs[i])

        # 既存測点を起点に
        if keep_existing and original_valid.iloc[i]:
            main, sub = _parse(name)
            offset = main * _SPAN + sub - x
            # 主測点なのに No. 無しを補正
            if sub == 0 and not name.startswith('No.'):
                names[i] = f'No.{main}'
            continue

        # 自動補完した測点を再計算させない
        if keep_existing and _is_valid(name):
            continue

        # 補完生成
        cum = x + offset
        names[i] = _name_from_dist(cum)

    out[name_col] = names
    return out


# ─────────────────────────
# 旧 API 互換ラッパー
# ─────────────────────────
def complete_station_names(df: pd.DataFrame, **kw):
    return fill_station_names(df, **kw)


def preserve_and_complete_station_names(df: pd.DataFrame, **kw):
    return fill_station_names(df, **kw)


# ----------------------------------------------------------------------------
# Legacy API wrappers and utilities
# ----------------------------------------------------------------------------

def generate_station_names(df: pd.DataFrame,
                           *,
                           name_column: str = 'name',
                           distance_column: str = 'x',
                           keep_existing: bool = True) -> pd.DataFrame:
    """Legacy helper that delegates to :func:`fill_station_names`.

    Parameters
    ----------
    df : pandas.DataFrame
        Input dataframe containing distance information.
    name_column : str, default 'name'
        Column used for station names.
    distance_column : str, default 'x'
        Column used for distances.
    keep_existing : bool, default True
        Whether to preserve already valid station names.

    Returns
    -------
    pandas.DataFrame
        DataFrame with station names filled in ``name_column``.
    """

    return fill_station_names(
        df,
        name_col=name_column,
        dist_col=distance_column,
        keep_existing=keep_existing,
    )


def round_to_precision(value, precision) -> pd.Series | float:
    """Round numeric values using ``Decimal`` for predictable results."""

    q = Decimal(str(precision))

    def _round_val(v):
        return float(Decimal(str(v)).quantize(q, ROUND_HALF_UP))

    if isinstance(value, pd.Series):
        return value.apply(_round_val)
    return _round_val(value)
