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
    """横断シートをパースする

    Excel構造:
    - col2 = 測点名
    - col3 = L標高（左端）
    - col4 = C標高（センター）
    - col5 = マンホール標高（オプション）
    - col6 = R標高（右端）
    - col8 = L距離（CLからの左幅）
    - col9 = R距離（CLからの右幅）
    - col10 = マンホール距離（オプション）
    """
    sections = []

    # 前回の距離を保持（欠損時のフォールバック用）
    prev_l_dist = 3.0
    prev_r_dist = 3.0

    # 行5から開始、2行ごとに1測点
    row_idx = 5
    while row_idx <= ws.max_row:
        station_name = ws.cell(row=row_idx, column=2).value
        if not station_name:
            row_idx += 2
            continue

        # 標高データを取得
        l_elev = ws.cell(row=row_idx, column=3).value   # L（左端）
        c_elev = ws.cell(row=row_idx, column=4).value   # C（センター）
        mh_elev = ws.cell(row=row_idx, column=5).value  # マンホール（オプション）
        r_elev = ws.cell(row=row_idx, column=6).value   # R（右端）

        # 距離データを取得
        l_dist_val = ws.cell(row=row_idx, column=8).value   # L距離
        r_dist_val = ws.cell(row=row_idx, column=9).value   # R距離
        mh_dist_val = ws.cell(row=row_idx, column=10).value # マンホール距離

        # 距離が欠損の場合は前回値を使用
        if l_dist_val is not None:
            l_dist = float(l_dist_val)
            prev_l_dist = l_dist
        else:
            l_dist = prev_l_dist

        if r_dist_val is not None:
            r_dist = float(r_dist_val)
            prev_r_dist = r_dist
        else:
            r_dist = prev_r_dist

        # データポイントを構築（左から右の順）
        points = []

        # L（左端）: 距離は負値
        if l_elev is not None:
            points.append((-l_dist, float(l_elev)))

        # C（センター）: 距離は0
        if c_elev is not None:
            points.append((0.0, float(c_elev)))

        # マンホール: 距離は正値
        if mh_elev is not None and mh_dist_val is not None:
            points.append((float(mh_dist_val), float(mh_elev)))

        # R（右端）: 距離は正値
        if r_elev is not None:
            points.append((r_dist, float(r_elev)))

        # 距離でソート
        points.sort(key=lambda p: p[0])

        if len(points) < 2:
            row_idx += 2
            continue

        # 距離と標高を分離
        distances = [p[0] for p in points]
        elevations = [p[1] for p in points]

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

        # SurveyRowを作成
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


def parse_keikaku_matome_sheet(ws, cutting_depth: float = 0.05) -> list[CrossSectionData]:
    """計画まとめシートをパースする

    Excel構造（行1がヘッダー）:
    - col2 = 測点名（現地盤高の行のみ）
    - col3 = 行種別（現地盤高/計画高/切削高/切削厚）
    - col4 = L標高, col5 = -4m, col6 = -2m, col7 = C, col8 = +2m, col9 = +4m, col10 = +6m, col11 = +8m, col12 = R
    - col14 = 左幅員, col15 = 右幅員
    - col17 = 左勾配, col18 = 右勾配

    5行で1測点（現地盤高, 計画高, 切削高, 切削厚, 空行）
    """
    sections = []
    seen_names = set()  # 重複検出用

    # 距離マッピング（col4-12の相対位置、後で実際の幅員で調整）
    # L=-幅員, -4, -2, C=0, +2, +4, +6, +8, R=+幅員
    dist_offsets = {
        4: None,   # L（左幅員で決まる）
        5: -4.0,
        6: -2.0,
        7: 0.0,    # C
        8: 2.0,
        9: 4.0,
        10: 6.0,
        11: 8.0,
        12: None,  # R（右幅員で決まる）
    }

    row_idx = 2  # 行2から開始（行1はヘッダー）
    while row_idx <= ws.max_row:
        station_name = ws.cell(row=row_idx, column=2).value
        row_type = ws.cell(row=row_idx, column=3).value

        if not station_name or row_type != "現地盤高":
            row_idx += 1
            continue

        # 重複チェック（2番目のルートに入ったら終了）
        name_str = str(station_name)
        if name_str in seen_names:
            break
        seen_names.add(name_str)

        # 幅員を取得
        l_width = ws.cell(row=row_idx, column=14).value
        r_width = ws.cell(row=row_idx, column=15).value

        if l_width is None or r_width is None:
            row_idx += 5
            continue

        l_width = float(l_width)
        r_width = float(r_width)

        # 現地盤高の行から標高を取得
        ground_elevations = {}
        for col in range(4, 13):
            val = ws.cell(row=row_idx, column=col).value
            if val is not None:
                ground_elevations[col] = float(val)

        # 計画高の行（次の行）から標高を取得
        planned_elevations = {}
        plan_row = row_idx + 1
        if ws.cell(row=plan_row, column=3).value == "計画高":
            for col in range(4, 13):
                val = ws.cell(row=plan_row, column=col).value
                if val is not None:
                    planned_elevations[col] = float(val)

        # データポイントを構築
        points = []
        for col, offset in dist_offsets.items():
            if col not in ground_elevations:
                continue

            # 距離を決定
            if col == 4:  # L
                dist = -l_width
            elif col == 12:  # R
                dist = r_width
            else:
                dist = offset
                # 幅員より外の点はスキップ
                if dist < -l_width or dist > r_width:
                    continue

            ground = ground_elevations[col]
            planned = planned_elevations.get(col, ground)

            points.append({
                'dist': dist,
                'ground': ground,
                'planned': planned,
            })

        # 距離でソート
        points.sort(key=lambda p: p['dist'])

        if len(points) < 2:
            row_idx += 5
            continue

        # CLのインデックスを見つける
        cl_index = 0
        for i, p in enumerate(points):
            if abs(p['dist']) < 0.001:
                cl_index = i
                break

        # 単延長を計算
        distances = [p['dist'] for p in points]
        unit_distances = [0.0]
        for i in range(1, len(distances)):
            unit_distances.append(distances[i] - distances[i-1])

        # DL（最小標高の小数点以下切り捨て）
        dl = math.floor(min(p['ground'] for p in points))

        # SurveyRowを作成
        survey_data = []
        for i, p in enumerate(points):
            survey_data.append(SurveyRow(
                unit_distance=unit_distances[i],
                elevation=p['ground'],
                planned_height=p['planned'],
                cumulative_distance=p['dist'],
                cutting_bottom=p['planned'] - cutting_depth,
            ))

        route_distance = parse_station_distance(str(station_name))

        sections.append(CrossSectionData(
            survey_point_name=str(station_name),
            dl=dl,
            cl_index=cl_index,
            l_to_cl_distance=l_width,
            survey_data=survey_data,
            route_distance=route_distance,
        ))

        row_idx += 5  # 次の測点へ

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

    # 横断データ - 計画まとめシートを優先使用
    if "計画まとめ" in wb.sheetnames:
        sections = parse_keikaku_matome_sheet(wb["計画まとめ"])
        result["cross_sections_merged"] = [asdict(s) for s in sections]
    else:
        # フォールバック: 横断(現況)と横断(計画)を使用
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
