#!/usr/bin/env python3
"""
計画.xlsx パーサー
縦断・横断データをJSON形式で出力する
"""

import json
import math
import re
import sys
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Optional

import openpyxl


@dataclass
class SurveyRow:
    """横断の1点データ"""
    unit_distance: float       # 単延長（間隔）
    elevation: float           # 現地盤高
    planned_height: float      # 計画高
    cumulative_distance: float # 累計距離（CLからの距離、左がマイナス）
    cutting_bottom: float      # 切削底面高


@dataclass
class CrossSectionData:
    """横断データ"""
    survey_point_name: str        # 測点名
    dl: float                     # DL（基準線高さ）
    cl_index: int                 # センターラインのインデックス
    l_to_cl_distance: float       # 左端からCLまでの距離
    survey_data: list[SurveyRow]  # 測量データ
    route_distance: Optional[float] = None  # 路線距離（m）


@dataclass
class LongitudinalPoint:
    """縦断の1点データ"""
    station_name: str      # 測点名
    ground_height: float   # 地盤高（現況）
    planned_height: float  # 計画高
    distance: float        # 追加距離
    cumulative: float      # 累積距離
    gradient: float        # 勾配


@dataclass
class LongitudinalSection:
    """縦断データ"""
    name: str                      # 路線名
    points: list[LongitudinalPoint]


def parse_station_distance(name: str) -> float:
    """測点名から路線距離を計算する
    例: No.0 -> 0, No.1 -> 20, No.1+10 -> 30, No.2+5.5 -> 45.5
    """
    # "No.X" or "No.X+Y" or "No.X+Yマンホール" のパターン
    match = re.match(r'No\.(\d+)(?:\+(\d+(?:\.\d+)?))?\s*', name)
    if not match:
        return 0.0

    no = int(match.group(1))
    plus = float(match.group(2)) if match.group(2) else 0.0
    # No.1 = 20m間隔と仮定
    return no * 20.0 + plus


def parse_longitudinal_sheet(ws, sheet_name: str) -> LongitudinalSection:
    """縦断シートをパースする"""
    points = []
    cumulative = 0.0

    # 行3から開始（行2がヘッダー）
    for row_idx in range(3, ws.max_row + 1):
        station_name = ws.cell(row=row_idx, column=2).value
        if not station_name:
            continue

        ground_height = ws.cell(row=row_idx, column=3).value
        if ground_height is None:
            continue

        distance = ws.cell(row=row_idx, column=4).value or 0.0
        gradient = ws.cell(row=row_idx, column=5).value or 0.0

        cumulative += distance

        # 計画高は縦断(計画)シートから取得するか、現況と同じにする
        planned_height = ground_height  # 後で上書き

        points.append(LongitudinalPoint(
            station_name=str(station_name),
            ground_height=float(ground_height),
            planned_height=float(planned_height),
            distance=float(distance),
            cumulative=cumulative,
            gradient=float(gradient) if gradient else 0.0,
        ))

    return LongitudinalSection(name=sheet_name, points=points)


def parse_cross_section_sheet(ws, sheet_name: str, cutting_depth: float = 0.05) -> list[CrossSectionData]:
    """横断シートをパースする"""
    sections = []

    # 行3がヘッダー: L, 6, 4, 2, C, マンホール, 2, 4, 6, 8, R
    # 横断位置（CLからの距離）を定義
    # 列: C(3), D(4)=-6, E(5)=-4, F(6)=-2, G(7)=C, H(8)=マンホール, I(9)=2, J(10)=4, K(11)=6, L(12)=8, M(13)=R

    # 列Oが左端距離、列Pが右端距離
    L_DIST_COL = 15  # O列
    R_DIST_COL = 16  # P列

    # 行5から開始、2行ごとに1測点（奇数行=標高、偶数行=勾配）
    row_idx = 5
    while row_idx <= ws.max_row:
        station_name = ws.cell(row=row_idx, column=2).value
        if not station_name:
            row_idx += 2
            continue

        # 左端・右端の距離を取得
        l_dist = ws.cell(row=row_idx, column=L_DIST_COL).value or 3.0
        r_dist = ws.cell(row=row_idx, column=R_DIST_COL).value or 3.0

        # 各列の標高を取得
        # 列C=L(左端), F=-2m, G=CL, H=マンホール, I=2m, J=4m, M=R(右端)
        col_mapping = [
            (3, -l_dist),   # L（左端）
            (6, -2.0),      # -2m
            (7, 0.0),       # CL
            (9, 2.0),       # +2m
            (10, 4.0),      # +4m
            (13, r_dist),   # R（右端）
        ]

        # データを収集（None以外）
        elevations = []
        distances = []
        for col, dist in col_mapping:
            val = ws.cell(row=row_idx, column=col).value
            if val is not None:
                elevations.append(float(val))
                distances.append(dist)

        if len(elevations) < 2:
            row_idx += 2
            continue

        # CLのインデックスを見つける
        cl_index = 0
        for i, d in enumerate(distances):
            if abs(d) < 0.001:
                cl_index = i
                break

        # 単延長を計算
        unit_distances = [0.0]
        for i in range(1, len(distances)):
            unit_distances.append(distances[i] - distances[i-1])

        # 左端からCLまでの距離
        l_to_cl = abs(distances[0]) if distances else 0.0

        # DL（最小標高の小数点以下切り捨て）
        dl = math.floor(min(elevations))

        # SurveyRowを作成（計画高は現況と同じ、あとで上書き可能）
        survey_data = []
        for i in range(len(elevations)):
            planned = elevations[i]  # 現況シートの場合は同じ値
            survey_data.append(SurveyRow(
                unit_distance=unit_distances[i],
                elevation=elevations[i],
                planned_height=planned,
                cumulative_distance=distances[i],
                cutting_bottom=planned - cutting_depth,
            ))

        route_distance = parse_station_distance(str(station_name))

        sections.append(CrossSectionData(
            survey_point_name=str(station_name),
            dl=dl,
            cl_index=cl_index,
            l_to_cl_distance=l_to_cl,
            survey_data=survey_data,
            route_distance=route_distance,
        ))

        row_idx += 2

    return sections


def parse_xlsx(xlsx_path: str) -> dict:
    """Excelファイルをパースして辞書形式で返す"""
    wb = openpyxl.load_workbook(xlsx_path, data_only=True)

    result = {
        "longitudinal_current": None,
        "longitudinal_planned": None,
        "cross_sections_current": [],
        "cross_sections_planned": [],
    }

    # 縦断データ
    if "縦断 (現況)" in wb.sheetnames:
        result["longitudinal_current"] = asdict(
            parse_longitudinal_sheet(wb["縦断 (現況)"], "縦断(現況)")
        )

    if "縦断 (計画)" in wb.sheetnames:
        result["longitudinal_planned"] = asdict(
            parse_longitudinal_sheet(wb["縦断 (計画)"], "縦断(計画)")
        )

    # 横断データ
    if "横断(現況)" in wb.sheetnames:
        sections = parse_cross_section_sheet(wb["横断(現況)"], "横断(現況)")
        result["cross_sections_current"] = [asdict(s) for s in sections]

    if "横断(計画)" in wb.sheetnames:
        sections = parse_cross_section_sheet(wb["横断(計画)"], "横断(計画)")
        result["cross_sections_planned"] = [asdict(s) for s in sections]

    # 現況と計画をマージ（計画高を上書き）
    if result["cross_sections_current"] and result["cross_sections_planned"]:
        current_map = {s["survey_point_name"]: s for s in result["cross_sections_current"]}
        planned_map = {s["survey_point_name"]: s for s in result["cross_sections_planned"]}

        merged = []
        for name, current in current_map.items():
            if name in planned_map:
                planned = planned_map[name]
                # 計画高を上書き
                for i, row in enumerate(current["survey_data"]):
                    if i < len(planned["survey_data"]):
                        row["planned_height"] = planned["survey_data"][i]["elevation"]
                        row["cutting_bottom"] = row["planned_height"] - 0.05
            merged.append(current)

        result["cross_sections_merged"] = merged

    return result


def to_cross_section_ui_format(data: dict) -> list[dict]:
    """cross-section-ui用のフォーマットに変換"""
    merged = data.get("cross_sections_merged", data.get("cross_sections_current", []))

    sections = []
    for s in merged:
        sections.append({
            "survey_point_name": s["survey_point_name"],
            "dl": s["dl"],
            "cl_index": s["cl_index"],
            "l_to_cl_distance": s["l_to_cl_distance"],
            "survey_data": s["survey_data"],
            "route_distance": s["route_distance"],
        })

    return sections


def main():
    if len(sys.argv) < 2:
        xlsx_path = Path(__file__).parent.parent / "data" / "計画.xlsx"
    else:
        xlsx_path = Path(sys.argv[1])

    if not xlsx_path.exists():
        print(f"Error: {xlsx_path} not found", file=sys.stderr)
        sys.exit(1)

    print(f"Parsing: {xlsx_path}", file=sys.stderr)

    data = parse_xlsx(str(xlsx_path))

    # cross-section-ui用に変換
    sections = to_cross_section_ui_format(data)

    # JSON出力（data/sections.jsonに保存）
    output_path = xlsx_path.parent / "sections.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(sections, f, ensure_ascii=False, indent=2)

    print(f"Output: {output_path}", file=sys.stderr)
    print(f"Sections: {len(sections)}", file=sys.stderr)

    # 確認用にサマリー出力
    for s in sections[:5]:
        print(f"  {s['survey_point_name']}: {len(s['survey_data'])} points, route={s['route_distance']}m", file=sys.stderr)
    if len(sections) > 5:
        print(f"  ... and {len(sections) - 5} more", file=sys.stderr)


if __name__ == "__main__":
    main()
