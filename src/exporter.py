# exporter.py
from pathlib import Path

import ezdxf
import pandas as pd

# 直接定数を定義（constants モジュールの依存を排除）
ROUND_N = 2        # 数値丸め桁数
DXF_UNITS = 6      # 6 = metres（ezdxf の INSUNITS）
DXF_MEAS = 1       # 1 = metric

def export_csv(df: pd.DataFrame, path: Path) -> None:
    df.to_csv(path, index=False, encoding="utf-8", float_format=f"%.{ROUND_N}f")

def export_dxf(df: pd.DataFrame, path: Path, draw_func) -> None:
    """draw_func(msp: ezdxf.layouts.Modelspace, df: DataFrame) を注入"""
    doc = ezdxf.new("R2010")
    doc.header["$INSUNITS"] = DXF_UNITS
    doc.header["$MEASUREMENT"] = DXF_MEAS

    draw_func(doc.modelspace(), df)
    doc.saveas(path)
