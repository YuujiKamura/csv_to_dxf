#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
出来形管理図（管理用紙）汎用PDF生成スクリプト

南千反畑町正本PDFのレイアウト・フォント・寸法比率に100%準拠し、
内挿点（測定点オフセットや標高・切削厚データ）の柔軟な差し替え・補間が可能な
A3横ベクターPDF生成エンジン。
"""

from __future__ import annotations

import argparse
import json
import math
import os
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

import fitz  # PyMuPDF
import openpyxl


# ============================================================================
# データモデル (Data Models)
# ============================================================================

@dataclass
class DekigataPoint:
    """出来形測定点データ"""
    label: str               # "V1", "V2", etc.
    offset: float            # CLからの横断距離(m)。左側は負、右側は正、CLは0.0
    gh: float                # 地盤高 (m)
    fh: float                # 計画高 (m)
    ch: float                # 切削高 (m)
    cut_mm: float            # 切削厚 (mm)

    @classmethod
    def create(
        cls,
        label: str,
        offset: float,
        gh: float,
        fh: float,
        ch: Optional[float] = None,
        cut_mm: Optional[float] = None,
        default_cut_mm: float = 50.0,
    ) -> DekigataPoint:
        if ch is None:
            ch = fh - (default_cut_mm / 1000.0)
        if cut_mm is None:
            cut_mm = (gh - ch) * 1000.0
        return cls(label=label, offset=offset, gh=gh, fh=fh, ch=ch, cut_mm=cut_mm)


@dataclass
class DekigataSection:
    """1測点の横断出来形データ"""
    station_name: str                 # "No.23", "No.25", etc.
    left_width: float                # 左幅員 (m)
    right_width: float               # 右幅員 (m)
    left_slope: float                # 左勾配 (%)、CLよりL側が低ければ負
    right_slope: float               # 右勾配 (%)、CLよりR側が低ければ負
    dl: float                        # 基準高 (m)
    points: List[DekigataPoint] = field(default_factory=list)
    project_name: str = ""           # 工事名（指定時は上部に表示）

    @property
    def cl_point(self) -> Optional[DekigataPoint]:
        """CL(中心)測定点を取得"""
        for pt in self.points:
            if abs(pt.offset) < 0.001:
                return pt
        if self.points:
            return min(self.points, key=lambda p: abs(p.offset))
        return None

    @property
    def left_point(self) -> Optional[DekigataPoint]:
        """左端測定点を取得"""
        return self.points[0] if self.points else None

    @property
    def right_point(self) -> Optional[DekigataPoint]:
        """右端測定点を取得"""
        return self.points[-1] if self.points else None

    def to_dict(self) -> Dict[str, Any]:
        """辞書に変換（JSONエクスポート用）"""
        return asdict(self)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> DekigataSection:
        """辞書から復元"""
        pts = [DekigataPoint(**p) for p in data.get("points", [])]
        return cls(
            station_name=data["station_name"],
            left_width=float(data["left_width"]),
            right_width=float(data["right_width"]),
            left_slope=float(data["left_slope"]),
            right_slope=float(data["right_slope"]),
            dl=float(data["dl"]),
            points=pts,
            project_name=data.get("project_name", ""),
        )


# ============================================================================
# 内挿・データ生成・差し替えエンジン (Interpolation & Data Manipulation)
# ============================================================================

class InterpolationEngine:
    """測定点の内挿・差し替え・再計算を行うエンジン"""

    @staticmethod
    def interpolate_heights(
        offset: float,
        base_points: List[DekigataPoint],
    ) -> Tuple[float, float, float]:
        """
        与えられたオフセットにおける (gh, fh, ch) を基準点から線形補間
        """
        sorted_pts = sorted(base_points, key=lambda p: p.offset)
        if offset <= sorted_pts[0].offset:
            p0 = sorted_pts[0]
            return p0.gh, p0.fh, p0.ch
        if offset >= sorted_pts[-1].offset:
            p1 = sorted_pts[-1]
            return p1.gh, p1.fh, p1.ch

        for i in range(len(sorted_pts) - 1):
            p_left = sorted_pts[i]
            p_right = sorted_pts[i + 1]
            if p_left.offset <= offset <= p_right.offset:
                span = p_right.offset - p_left.offset
                if span < 1e-6:
                    return p_left.gh, p_left.fh, p_left.ch
                t = (offset - p_left.offset) / span
                gh = p_left.gh + t * (p_right.gh - p_left.gh)
                fh = p_left.fh + t * (p_right.fh - p_left.fh)
                ch = p_left.ch + t * (p_right.ch - p_left.ch)
                return gh, fh, ch

        return sorted_pts[-1].gh, sorted_pts[-1].fh, sorted_pts[-1].ch

    @classmethod
    def resample_points(
        cls,
        section: DekigataSection,
        new_offsets: List[float],
        default_cut_mm: float = 50.0,
    ) -> DekigataSection:
        """
        指定した任意のオフセットリストで測定点を再サンプリング（差し替え）
        """
        if not section.points:
            return section

        new_pts: List[DekigataPoint] = []
        for i, off in enumerate(sorted(new_offsets)):
            v_label = f"V{i + 1}"
            gh, fh, ch = cls.interpolate_heights(off, section.points)
            cut_mm = (gh - ch) * 1000.0
            new_pts.append(DekigataPoint(
                label=v_label,
                offset=off,
                gh=gh,
                fh=fh,
                ch=ch,
                cut_mm=cut_mm,
            ))

        return DekigataSection(
            station_name=section.station_name,
            left_width=section.left_width,
            right_width=section.right_width,
            left_slope=section.left_slope,
            right_slope=section.right_slope,
            dl=section.dl,
            points=new_pts,
            project_name=section.project_name,
        )

    @classmethod
    def override_points(
        cls,
        section: DekigataSection,
        overrides: Dict[str, Dict[str, float]],
    ) -> DekigataSection:
        """
        特定の点（例: 'V2', 'V4'）の属性（gh, fh, ch, cut_mm等）を上書き
        """
        updated_pts = []
        for pt in section.points:
            if pt.label in overrides:
                o = overrides[pt.label]
                new_pt = DekigataPoint(
                    label=pt.label,
                    offset=o.get("offset", pt.offset),
                    gh=o.get("gh", pt.gh),
                    fh=o.get("fh", pt.fh),
                    ch=o.get("ch", pt.ch),
                    cut_mm=o.get("cut_mm", pt.cut_mm),
                )
                updated_pts.append(new_pt)
            else:
                updated_pts.append(pt)

        return DekigataSection(
            station_name=section.station_name,
            left_width=section.left_width,
            right_width=section.right_width,
            left_slope=section.left_slope,
            right_slope=section.right_slope,
            dl=section.dl,
            points=updated_pts,
            project_name=section.project_name,
        )


# ============================================================================
# Excelデータローダー (Excel Loader)
# ============================================================================

def calc_dl_value(min_height: float) -> float:
    """最小標高からキリの良いDL基準高を決定"""
    dl = math.floor(min_height)
    if (min_height - dl) < 0.2:
        dl -= 1.0
    return float(dl)


def load_sections_from_keikaku_matome(
    xlsx_path: Path | str,
    target_stations: Optional[List[str]] = None,
    sheet_name: str = "計画まとめ",
    custom_offsets_func: Optional[Callable[[DekigataSection], List[float]]] = None,
    point_modifier: Optional[Callable[[DekigataSection], DekigataSection]] = None,
    project_name: str = "",
) -> List[DekigataSection]:
    """
    計画まとめ.xlsx から測点データを読み込み、DekigataSection オブジェクトを構築

    内挿差し替えフック:
    - `custom_offsets_func`: 測点ごとに内挿オフセットを差し替える関数
    - `point_modifier`: 読み込み後に各点の値を上書き・加工する関数
    """
    xlsx_path = Path(xlsx_path)
    if not xlsx_path.exists():
        raise FileNotFoundError(f"Excel file not found: {xlsx_path}")

    wb = openpyxl.load_workbook(xlsx_path, data_only=True)
    if sheet_name not in wb.sheetnames:
        sheet = wb.active
    else:
        sheet = wb[sheet_name]

    sections: List[DekigataSection] = []

    for r in range(1, sheet.max_row + 1):
        cell_b = sheet.cell(r, 2).value
        if not cell_b:
            continue
        st_name = str(cell_b).strip()
        if target_stations is not None and st_name not in target_stations:
            continue

        lw_val = sheet.cell(r, 11).value
        rw_val = sheet.cell(r, 12).value
        slope_val = sheet.cell(r, 14).value

        if lw_val is None or rw_val is None:
            continue

        left_width = float(lw_val)
        right_width = float(rw_val)
        base_slope = float(slope_val) if slope_val is not None else 0.02

        # 4行構成 (r: GH, r+1: FH, r+2: CH, r+3: Cut)
        # 列マッピング: 4: L (-lw), 5: -2.0, 6: C (0.0), 7: +2.0, 9: R (+rw)
        col_specs = [
            (4, "V1", -left_width),
            (5, "V2", -2.0),
            (6, "V3", 0.0),
            (7, "V4", 2.0),
            (9, "V5", right_width),
        ]

        pts: List[DekigataPoint] = []
        min_elev = 99999.0

        for col_idx, v_label, off in col_specs:
            gh = sheet.cell(r, col_idx).value
            fh = sheet.cell(r + 1, col_idx).value
            ch = sheet.cell(r + 2, col_idx).value
            cut = sheet.cell(r + 3, col_idx).value

            if gh is None or fh is None:
                continue

            gh = float(gh)
            fh = float(fh)
            ch = float(ch) if ch is not None else fh - 0.05
            cut_mm = float(cut) * 1000.0 if cut is not None else (gh - ch) * 1000.0

            min_elev = min(min_elev, gh, fh, ch)
            pts.append(DekigataPoint(
                label=v_label,
                offset=off,
                gh=gh,
                fh=fh,
                ch=ch,
                cut_mm=cut_mm,
            ))

        if len(pts) < 2:
            continue

        pt_l = pts[0]
        pt_c = next((p for p in pts if abs(p.offset) < 0.001), pts[len(pts)//2])
        pt_r = pts[-1]

        l_span = abs(pt_c.offset - pt_l.offset)
        r_span = abs(pt_r.offset - pt_c.offset)
        l_slope = ((pt_l.fh - pt_c.fh) / l_span * 100.0) if l_span > 0 else -base_slope * 100.0
        r_slope = ((pt_r.fh - pt_c.fh) / r_span * 100.0) if r_span > 0 else base_slope * 100.0

        dl = calc_dl_value(min_elev)

        section = DekigataSection(
            station_name=st_name,
            left_width=left_width,
            right_width=right_width,
            left_slope=l_slope,
            right_slope=r_slope,
            dl=dl,
            points=pts,
            project_name=project_name,
        )

        # 内挿差し替えフックの適用
        if custom_offsets_func is not None:
            new_offs = custom_offsets_func(section)
            section = InterpolationEngine.resample_points(section, new_offs)

        if point_modifier is not None:
            section = point_modifier(section)

        sections.append(section)

    return sections


# ============================================================================
# PDF 描画エンジン (PyMuPDF Vector Renderer)
# ============================================================================

class DekigataPdfRenderer:
    """
    出来形管理図A3横PDFレンダラー
    南千反畑町正本PDFと完全一致する座標・線幅・比率でベクター描画を行う。
    """

    PAGE_WIDTH = 1190.52   # A3横幅 (pt)
    PAGE_HEIGHT = 841.92   # A3縦幅 (pt)
    MARGIN = 28.32         # 外枠マージン 10mm (pt)

    SCALE_H = 56.6929      # 水平1:50 (pt/m, 1m = 20mm = 56.6929pt)
    SCALE_V = 56.6929      # 垂直1:50 (pt/m)

    COLOR_BLACK = (0.0, 0.0, 0.0)
    COLOR_RED = (1.0, 0.0, 0.0)
    COLOR_BLUE = (0.0, 0.0, 1.0)
    COLOR_GRAY = (0.502, 0.502, 0.502)

    LINE_WIDTH = 0.375     # 標準線幅 (pt)

    def __init__(self, font_path: str = r"C:\Windows\Fonts\msgothic.ttc"):
        self.font_path = font_path
        self.font_name = "msgothic"
        self.font = fitz.Font(fontfile=font_path)

    def _get_text_width(self, text: str, fontsize: float) -> float:
        """テキスト幅を正確に計算"""
        return self.font.text_length(text, fontsize=fontsize)

    def render_section_to_page(
        self,
        page: fitz.Page,
        section: DekigataSection,
        v_scale_ratio: float = 1.0,
    ) -> None:
        """単一測点の出来形管理図をページに描画"""
        page.insert_font(fontname=self.font_name, fontfile=self.font_path)

        cl_x = self.PAGE_WIDTH / 2.0  # 595.26 pt (横断図の中央CL)
        dl_y = 309.8                   # DL線のY座標 (pt)
        v_scale = self.SCALE_V * v_scale_ratio

        # -------------------------------------------------------------
        # 1. 外枠 (Outer Border)
        # -------------------------------------------------------------
        border_rect = fitz.Rect(
            self.MARGIN,
            self.MARGIN,
            self.PAGE_WIDTH - self.MARGIN,
            self.PAGE_HEIGHT - self.MARGIN,
        )
        page.draw_rect(border_rect, color=self.COLOR_BLACK, width=self.LINE_WIDTH)

        # -------------------------------------------------------------
        # 2. 上部表題 (Main Title: Huge Project Name)
        # -------------------------------------------------------------
        # 「出来形管理用紙」の文言を削除し、工事名をバカでかくメインタイトルとして配置
        title_text = section.project_name if section.project_name else "出来形管理用紙"
        title_size = 32.0 if section.project_name else 24.0
        title_w = self._get_text_width(title_text, title_size)
        title_x = cl_x - (title_w / 2.0)
        title_y = 66.0
        page.insert_text(fitz.Point(title_x, title_y), title_text, fontname=self.font_name, fontsize=title_size, color=self.COLOR_BLACK)

        # 二重下線 (Double Underline)
        ul_w = title_w + 20.0
        ul_x0 = cl_x - (ul_w / 2.0)
        ul_x1 = cl_x + (ul_w / 2.0)
        ul_y1, ul_y2 = 74.0, 76.8
        page.draw_line(fitz.Point(ul_x0, ul_y1), fitz.Point(ul_x1, ul_y1), color=self.COLOR_BLACK, width=self.LINE_WIDTH)
        page.draw_line(fitz.Point(ul_x0, ul_y2), fitz.Point(ul_x1, ul_y2), color=self.COLOR_BLACK, width=self.LINE_WIDTH)

        # -------------------------------------------------------------
        # 3. 測点名 (Station Name)
        # -------------------------------------------------------------
        st_text = section.station_name
        st_font_size = 24.3
        st_w = self._get_text_width(st_text, st_font_size)
        st_x = cl_x - (st_w / 2.0)
        st_y = 105.0
        page.insert_text(fitz.Point(st_x, st_y), st_text, fontname=self.font_name, fontsize=st_font_size, color=self.COLOR_BLACK)

        # -------------------------------------------------------------
        # 4. 横断図データ準備 (Coordinates)
        # -------------------------------------------------------------
        pt_coords: List[Tuple[DekigataPoint, float, float, float, float]] = []
        for pt in section.points:
            px = cl_x + pt.offset * self.SCALE_H
            gh_y = dl_y - (pt.gh - section.dl) * v_scale
            fh_y = dl_y - (pt.fh - section.dl) * v_scale
            ch_y = dl_y - (pt.ch - section.dl) * v_scale
            pt_coords.append((pt, px, gh_y, fh_y, ch_y))

        l_coord = pt_coords[0]
        r_coord = pt_coords[-1]
        cl_coord = min(pt_coords, key=lambda c: abs(c[0].offset))

        # -------------------------------------------------------------
        # 5. 中央旗揚げ & 左右寸法・勾配 (CL Flag & Dimensions)
        # -------------------------------------------------------------
        cl_pt = cl_coord[0]
        cl_gh_str = f"GH={cl_pt.gh:.3f}"
        cl_fh_str = f"FH={cl_pt.fh:.3f}"
        font_s = 15.0

        w_gh = self._get_text_width(cl_gh_str, font_s)
        w_fh = self._get_text_width(cl_fh_str, font_s)
        page.insert_text(fitz.Point(cl_x - w_gh / 2.0, 128.0), cl_gh_str, fontname=self.font_name, fontsize=font_s, color=self.COLOR_BLACK)
        page.insert_text(fitz.Point(cl_x - w_fh / 2.0, 148.0), cl_fh_str, fontname=self.font_name, fontsize=font_s, color=self.COLOR_RED)

        # 水平寸法線 (旗揚げ基準線)
        flag_line_y = 165.0
        page.draw_line(fitz.Point(l_coord[1], flag_line_y), fitz.Point(cl_x, flag_line_y), color=self.COLOR_BLACK, width=self.LINE_WIDTH)
        page.draw_line(fitz.Point(cl_x, flag_line_y), fitz.Point(r_coord[1], flag_line_y), color=self.COLOR_BLACK, width=self.LINE_WIDTH)

        # 垂直チック線 (左端, CL, 右端)
        tick_top = 162.0
        tick_bot = 188.0
        page.draw_line(fitz.Point(l_coord[1], tick_top), fitz.Point(l_coord[1], tick_bot), color=self.COLOR_BLACK, width=self.LINE_WIDTH)
        page.draw_line(fitz.Point(cl_x, tick_top), fitz.Point(cl_x, tick_bot), color=self.COLOR_BLACK, width=self.LINE_WIDTH)
        page.draw_line(fitz.Point(r_coord[1], tick_top), fitz.Point(r_coord[1], tick_bot), color=self.COLOR_BLACK, width=self.LINE_WIDTH)

        # 幅員テキスト (旗揚げラインの上)
        l_mid_x = (l_coord[1] + cl_x) / 2.0
        r_mid_x = (cl_x + r_coord[1]) / 2.0

        lw_str = f"{section.left_width:.2f}"
        rw_str = f"{section.right_width:.2f}"
        w_lw = self._get_text_width(lw_str, font_s)
        w_rw = self._get_text_width(rw_str, font_s)
        page.insert_text(fitz.Point(l_mid_x - w_lw / 2.0, 158.0), lw_str, fontname=self.font_name, fontsize=font_s, color=self.COLOR_BLACK)
        page.insert_text(fitz.Point(r_mid_x - w_rw / 2.0, 158.0), rw_str, fontname=self.font_name, fontsize=font_s, color=self.COLOR_BLACK)

        # 勾配テキスト & 矢印 (幅員旗揚ラインの下)
        ls_str = f"{section.left_slope:.1f}%"
        rs_str = f"{section.right_slope:.1f}%"
        slope_font_s = 13.5
        w_ls = self._get_text_width(ls_str, slope_font_s)
        w_rs = self._get_text_width(rs_str, slope_font_s)
        slope_text_y = 179.0
        slope_arrow_y = 187.0
        page.insert_text(fitz.Point(l_mid_x - w_ls / 2.0, slope_text_y), ls_str, fontname=self.font_name, fontsize=slope_font_s, color=self.COLOR_BLACK)
        page.insert_text(fitz.Point(r_mid_x - w_rs / 2.0, slope_text_y), rs_str, fontname=self.font_name, fontsize=slope_font_s, color=self.COLOR_BLACK)

        self._draw_slope_arrow(page, l_mid_x, slope_arrow_y, section.left_slope, is_left_side=True)
        self._draw_slope_arrow(page, r_mid_x, slope_arrow_y, section.right_slope, is_left_side=False)

        # -------------------------------------------------------------
        # 6. 左右端点旗揚げ (Left & Right Flags) - 勾配と干渉しないよう配置
        # -------------------------------------------------------------
        l_gh_str = f"GH={l_coord[0].gh:.3f}"
        l_fh_str = f"FH={l_coord[0].fh:.3f}"
        page.insert_text(fitz.Point(l_coord[1], 206.0), l_gh_str, fontname=self.font_name, fontsize=font_s, color=self.COLOR_BLACK)
        page.insert_text(fitz.Point(l_coord[1], 226.0), l_fh_str, fontname=self.font_name, fontsize=font_s, color=self.COLOR_RED)

        r_gh_str = f"GH={r_coord[0].gh:.3f}"
        r_fh_str = f"FH={r_coord[0].fh:.3f}"
        w_rgh = self._get_text_width(r_gh_str, font_s)
        w_rfh = self._get_text_width(r_fh_str, font_s)
        page.insert_text(fitz.Point(r_coord[1] - w_rgh, 206.0), r_gh_str, fontname=self.font_name, fontsize=font_s, color=self.COLOR_BLACK)
        page.insert_text(fitz.Point(r_coord[1] - w_rfh, 226.0), r_fh_str, fontname=self.font_name, fontsize=font_s, color=self.COLOR_RED)

        # -------------------------------------------------------------
        # 7. 断面線 (Cross-section Lines: GH, FH, CH)
        # -------------------------------------------------------------
        for i in range(len(pt_coords) - 1):
            p0 = pt_coords[i]
            p1 = pt_coords[i + 1]

            page.draw_line(fitz.Point(p0[1], p0[2]), fitz.Point(p1[1], p1[2]), color=self.COLOR_BLACK, width=self.LINE_WIDTH)
            page.draw_line(fitz.Point(p0[1], p0[3]), fitz.Point(p1[1], p1[3]), color=self.COLOR_RED, width=self.LINE_WIDTH)
            page.draw_line(fitz.Point(p0[1], p0[4]), fitz.Point(p1[1], p1[4]), color=self.COLOR_BLUE, width=self.LINE_WIDTH)

        # -------------------------------------------------------------
        # 8. 測定点ポインタ (▽ V1〜Vn)
        # -------------------------------------------------------------
        tri_h = 8.5
        tri_hw = 5.1
        for pt, px, _, fh_y, _ in pt_coords:
            p_top_l = fitz.Point(px - tri_hw, fh_y - tri_h)
            p_top_r = fitz.Point(px + tri_hw, fh_y - tri_h)
            p_bottom = fitz.Point(px, fh_y)

            page.draw_line(p_top_l, p_top_r, color=self.COLOR_BLUE, width=self.LINE_WIDTH)
            page.draw_line(p_top_l, p_bottom, color=self.COLOR_BLUE, width=self.LINE_WIDTH)
            page.draw_line(p_top_r, p_bottom, color=self.COLOR_BLUE, width=self.LINE_WIDTH)

            lbl_w = self._get_text_width(pt.label, font_s)
            page.insert_text(fitz.Point(px - lbl_w / 2.0, fh_y - 12.0), pt.label, fontname=self.font_name, fontsize=font_s, color=self.COLOR_BLUE)

        # -------------------------------------------------------------
        # 9. DL基準線 & 切削厚表示 (DL Line & Cutting Depths)
        # -------------------------------------------------------------
        page.draw_line(fitz.Point(l_coord[1], dl_y), fitz.Point(r_coord[1], dl_y), color=self.COLOR_GRAY, width=self.LINE_WIDTH)
        page.draw_line(fitz.Point(cl_x, dl_y), fitz.Point(cl_x, cl_coord[4]), color=self.COLOR_GRAY, width=self.LINE_WIDTH)

        scale_txt = "Scale:H1:V2"
        dl_str = f"DL={section.dl:.3f}  {scale_txt}"
        page.insert_text(fitz.Point(cl_x, dl_y - 2.0), dl_str, fontname=self.font_name, fontsize=8.1, color=self.COLOR_GRAY)

        cut_y = 335.0
        for pt, px, _, _, _ in pt_coords:
            c_str = f"{pt.cut_mm:.0f}"
            c_w = self._get_text_width(c_str, font_s)
            page.insert_text(fitz.Point(px - c_w / 2.0, cut_y), c_str, fontname=self.font_name, fontsize=font_s, color=self.COLOR_BLUE)

        cut_lbl = "切削厚"
        w_cutlbl = self._get_text_width(cut_lbl, font_s)
        page.insert_text(fitz.Point(cl_x - w_cutlbl / 2.0, 358.0), cut_lbl, fontname=self.font_name, fontsize=font_s, color=self.COLOR_BLUE)

        # -------------------------------------------------------------
        # 10. 出来形管理表 (Dekigata Table: 5 rows x N cols)
        # -------------------------------------------------------------
        num_pts = len(section.points)
        tbl_hdr_w = 151.7
        tbl_data_w = 113.8
        tbl_total_w = tbl_hdr_w + tbl_data_w * num_pts
        tbl_x0 = cl_x - (tbl_total_w / 2.0)
        tbl_y0 = 380.8
        row_h = 50.56
        tbl_y_end = tbl_y0 + row_h * 5.0
        tbl_font_s = 24.2

        # 外枠
        page.draw_rect(fitz.Rect(tbl_x0, tbl_y0, tbl_x0 + tbl_total_w, tbl_y_end), color=self.COLOR_BLACK, width=self.LINE_WIDTH)

        # 横線 (4本)
        for r_i in range(1, 5):
            ry = tbl_y0 + row_h * r_i
            page.draw_line(fitz.Point(tbl_x0, ry), fitz.Point(tbl_x0 + tbl_total_w, ry), color=self.COLOR_BLACK, width=self.LINE_WIDTH)

        # 縦線
        page.draw_line(fitz.Point(tbl_x0 + tbl_hdr_w, tbl_y0), fitz.Point(tbl_x0 + tbl_hdr_w, tbl_y_end), color=self.COLOR_BLACK, width=self.LINE_WIDTH)
        for c_i in range(1, num_pts):
            cx = tbl_x0 + tbl_hdr_w + tbl_data_w * c_i
            page.draw_line(fitz.Point(cx, tbl_y0), fitz.Point(cx, tbl_y_end), color=self.COLOR_BLACK, width=self.LINE_WIDTH)

        # 行ラベル
        row_labels = [
            ("", self.COLOR_BLACK),
            ("計画高(設計)", self.COLOR_BLACK),
            ("計画高(実施)", self.COLOR_RED),
            ("切削高(設計)", self.COLOR_BLACK),
            ("切削高(実施)", self.COLOR_RED),
        ]
        for r_i, (lbl, col) in enumerate(row_labels):
            if not lbl:
                continue
            lw = self._get_text_width(lbl, tbl_font_s)
            lx = tbl_x0 + (tbl_hdr_w - lw) / 2.0
            ly = tbl_y0 + row_h * r_i + (row_h + tbl_font_s * 0.75) / 2.0 - 2.0
            page.insert_text(fitz.Point(lx, ly), lbl, fontname=self.font_name, fontsize=tbl_font_s, color=col)

        # 列データ
        for c_i, pt in enumerate(section.points):
            col_cx = tbl_x0 + tbl_hdr_w + tbl_data_w * (c_i + 0.5)

            # 1行目: Vn
            v_w = self._get_text_width(pt.label, tbl_font_s)
            v_y = tbl_y0 + (row_h + tbl_font_s * 0.75) / 2.0 - 2.0
            page.insert_text(fitz.Point(col_cx - v_w / 2.0, v_y), pt.label, fontname=self.font_name, fontsize=tbl_font_s, color=self.COLOR_BLUE)

            # 2行目: 計画高(設計)
            fh_str = f"{pt.fh:.3f}"
            fh_w = self._get_text_width(fh_str, tbl_font_s)
            fh_text_y = tbl_y0 + row_h * 1 + (row_h + tbl_font_s * 0.75) / 2.0 - 2.0
            page.insert_text(fitz.Point(col_cx - fh_w / 2.0, fh_text_y), fh_str, fontname=self.font_name, fontsize=tbl_font_s, color=self.COLOR_BLACK)

            # 3行目: 計画高(実施) -> 手書き空欄

            # 4行目: 切削高(設計)
            ch_str = f"{pt.ch:.3f}"
            ch_w = self._get_text_width(ch_str, tbl_font_s)
            ch_text_y = tbl_y0 + row_h * 3 + (row_h + tbl_font_s * 0.75) / 2.0 - 2.0
            page.insert_text(fitz.Point(col_cx - ch_w / 2.0, ch_text_y), ch_str, fontname=self.font_name, fontsize=tbl_font_s, color=self.COLOR_BLACK)

            # 5行目: 切削高(実施) -> 手書き空欄

        # -------------------------------------------------------------
        # 11. 幅員表 (Width Table: 3 rows x 2 cols)
        # -------------------------------------------------------------
        fuk_hdr_w = 151.7
        fuk_data_w = 126.4
        fuk_total_w = fuk_hdr_w + fuk_data_w * 2.0
        fuk_x0 = cl_x - (fuk_total_w / 2.0)
        fuk_y0 = 647.8
        fuk_row_h = 50.53
        fuk_y_end = fuk_y0 + fuk_row_h * 3.0

        # 外枠
        page.draw_rect(fitz.Rect(fuk_x0, fuk_y0, fuk_x0 + fuk_total_w, fuk_y_end), color=self.COLOR_BLACK, width=self.LINE_WIDTH)

        # 横線 (2本)
        for r_i in range(1, 3):
            ry = fuk_y0 + fuk_row_h * r_i
            page.draw_line(fitz.Point(fuk_x0, ry), fitz.Point(fuk_x0 + fuk_total_w, ry), color=self.COLOR_BLACK, width=self.LINE_WIDTH)

        # 縦線
        page.draw_line(fitz.Point(fuk_x0 + fuk_hdr_w, fuk_y0), fitz.Point(fuk_x0 + fuk_hdr_w, fuk_y_end), color=self.COLOR_BLACK, width=self.LINE_WIDTH)
        page.draw_line(fitz.Point(fuk_x0 + fuk_hdr_w + fuk_data_w, fuk_y0), fitz.Point(fuk_x0 + fuk_hdr_w + fuk_data_w, fuk_y_end), color=self.COLOR_BLACK, width=self.LINE_WIDTH)

        # 行ラベル
        fuk_row_labels = [
            ("", self.COLOR_BLACK),
            ("設計", self.COLOR_BLACK),
            ("実測", self.COLOR_RED),
        ]
        for r_i, (lbl, col) in enumerate(fuk_row_labels):
            if not lbl:
                continue
            lw = self._get_text_width(lbl, tbl_font_s)
            lx = fuk_x0 + (fuk_hdr_w - lw) / 2.0
            ly = fuk_y0 + fuk_row_h * r_i + (fuk_row_h + tbl_font_s * 0.75) / 2.0 - 2.0
            page.insert_text(fitz.Point(lx, ly), lbl, fontname=self.font_name, fontsize=tbl_font_s, color=col)

        # 列ヘッダー
        col_l_cx = fuk_x0 + fuk_hdr_w + fuk_data_w * 0.5
        col_r_cx = fuk_x0 + fuk_hdr_w + fuk_data_w * 1.5

        w_lf = self._get_text_width("左幅員", tbl_font_s)
        w_rf = self._get_text_width("右幅員", tbl_font_s)
        fuk_hdr_y = fuk_y0 + (fuk_row_h + tbl_font_s * 0.75) / 2.0 - 2.0
        page.insert_text(fitz.Point(col_l_cx - w_lf / 2.0, fuk_hdr_y), "左幅員", fontname=self.font_name, fontsize=tbl_font_s, color=self.COLOR_BLACK)
        page.insert_text(fitz.Point(col_r_cx - w_rf / 2.0, fuk_hdr_y), "右幅員", fontname=self.font_name, fontsize=tbl_font_s, color=self.COLOR_BLACK)

        # 設計値
        fuk_des_y = fuk_y0 + fuk_row_h * 1 + (fuk_row_h + tbl_font_s * 0.75) / 2.0 - 2.0
        page.insert_text(fitz.Point(col_l_cx - w_lw / 2.0, fuk_des_y), lw_str, fontname=self.font_name, fontsize=tbl_font_s, color=self.COLOR_BLACK)
        page.insert_text(fitz.Point(col_r_cx - w_rw / 2.0, fuk_des_y), rw_str, fontname=self.font_name, fontsize=tbl_font_s, color=self.COLOR_BLACK)

    def _draw_slope_arrow(
        self,
        page: fitz.Page,
        mid_x: float,
        y: float,
        slope: float,
        is_left_side: bool,
    ) -> None:
        """水勾配矢印を描画（高い方から低い方へ流れる向き）"""
        arrow_len = 34.0
        arrow_drop = 2.2
        head_len = 10.2
        head_w = 4.0

        if is_left_side:
            points_left = (slope < 0.0)
        else:
            points_left = (slope > 0.0)

        half_l = arrow_len / 2.0
        if points_left:
            p_start = fitz.Point(mid_x + half_l, y - arrow_drop)
            p_end = fitz.Point(mid_x - half_l, y + arrow_drop)
            p_head = fitz.Point(mid_x - half_l + head_len, y + arrow_drop - head_w)
            page.draw_line(p_start, p_end, color=self.COLOR_BLACK, width=self.LINE_WIDTH)
            page.draw_line(p_end, p_head, color=self.COLOR_BLACK, width=self.LINE_WIDTH)
        else:
            p_start = fitz.Point(mid_x - half_l, y - arrow_drop)
            p_end = fitz.Point(mid_x + half_l, y + arrow_drop)
            p_head = fitz.Point(mid_x + half_l - head_len, y + arrow_drop - head_w)
            page.draw_line(p_start, p_end, color=self.COLOR_BLACK, width=self.LINE_WIDTH)
            page.draw_line(p_end, p_head, color=self.COLOR_BLACK, width=self.LINE_WIDTH)

    def render_to_pdf(
        self,
        sections: List[DekigataSection],
        output_pdf_path: Path | str,
        v_scale_ratio: float = 1.0,
        output_individual: bool = False,
    ) -> List[Path]:
        """複数測点を結合したPDF（および個別PDF）を生成"""
        output_path = Path(output_pdf_path)
        output_path.parent.mkdir(parents=True, exist_ok=True)

        generated_paths: List[Path] = []

        # 1. 結合PDF
        doc = fitz.open()
        for sec in sections:
            page = doc.new_page(width=self.PAGE_WIDTH, height=self.PAGE_HEIGHT)
            self.render_section_to_page(page, sec, v_scale_ratio=v_scale_ratio)

        doc.save(str(output_path))
        doc.close()
        generated_paths.append(output_path)

        # 2. 個別PDF
        if output_individual:
            indiv_dir = output_path.parent / "個別"
            indiv_dir.mkdir(parents=True, exist_ok=True)
            for sec in sections:
                # Use station_name directly, e.g. 出来形管理図_No.23.pdf
                safe_name = sec.station_name.replace("+", "_")
                indiv_path = indiv_dir / f"出来形管理図_{safe_name}.pdf"
                indiv_doc = fitz.open()
                indiv_page = indiv_doc.new_page(width=self.PAGE_WIDTH, height=self.PAGE_HEIGHT)
                self.render_section_to_page(indiv_page, sec, v_scale_ratio=v_scale_ratio)
                indiv_doc.save(str(indiv_path))
                indiv_doc.close()
                generated_paths.append(indiv_path)

        return generated_paths


# ============================================================================
# CLI エントリーポイント
# ============================================================================

def main():
    parser = argparse.ArgumentParser(description="出来形管理図 汎用PDF生成ツール")
    parser.add_argument("excel", nargs="?", default=None, help="計画まとめ.xlsx パス")
    parser.add_argument("--stations", "-s", nargs="+", default=None, help="対象測点リスト (例: No.23 No.25 ...)")
    parser.add_argument("--output", "-o", default=None, help="出力PDFパス")
    parser.add_argument("--sheet", default="計画まとめ", help="Excelシート名")
    parser.add_argument("--v-ratio", type=float, default=1.0, help="縦方向スケール倍率 (デフォルト: 1.0)")
    parser.add_argument("--project-name", "-p", default="一般県道　熊本空港線（戸島西工区）舗装補修工事（２工区）", help="工事名（指定時は上部に表示）")
    parser.add_argument("--offsets", nargs="+", type=float, default=None, help="任意の内挿オフセットリストで全測点を再サンプリング (例: -3.0 -1.5 0.0 1.5 3.0)")
    parser.add_argument("--points-json", default=None, help="外部JSONから特定測点の測定点データを上書き")
    parser.add_argument("--individual", action="store_true", help="測点ごとの個別PDFも出力")
    args = parser.parse_args()

    default_excel = Path(r"I:\マイドライブ\〇一般県道　熊本空港線（戸島西工区）舗装補修工事（２工区）\一般県道　熊本空港線（戸島西工区）舗装補修工事（２工区）\１０測量と設計照査\計画まとめ.xlsx")
    excel_path = Path(args.excel) if args.excel else default_excel

    target_stations = args.stations
    if not target_stations:
        target_stations = ["No.23", "No.25", "No.27", "No.29", "No.31", "No.33"]

    print(f"Loading stations {target_stations} from: {excel_path}")

    # 内挿オフセット差し替えフック
    custom_offsets_func = None
    if args.offsets:
        custom_offsets_func = lambda sec: args.offsets

    # 外部JSONによる上書きフック
    point_modifier = None
    if args.points_json:
        with open(args.points_json, "r", encoding="utf-8") as fp:
            overrides_dict = json.load(fp)

        def modifier(sec: DekigataSection) -> DekigataSection:
            if sec.station_name in overrides_dict:
                return InterpolationEngine.override_points(sec, overrides_dict[sec.station_name])
            return sec

        point_modifier = modifier

    sections = load_sections_from_keikaku_matome(
        excel_path,
        target_stations=target_stations,
        sheet_name=args.sheet,
        custom_offsets_func=custom_offsets_func,
        point_modifier=point_modifier,
        project_name=args.project_name,
    )

    if not sections:
        print("Error: No sections loaded.")
        sys.exit(1)

    print(f"Successfully loaded {len(sections)} sections.")

    renderer = DekigataPdfRenderer()
    if args.output:
        out_pdf = Path(args.output)
    else:
        out_pdf = Path(r"I:\マイドライブ\〇一般県道　熊本空港線（戸島西工区）舗装補修工事（２工区）\一般県道　熊本空港線（戸島西工区）舗装補修工事（２工区）\１６出来形管理\出来形管理図_戸島西工区_奇数測点.pdf")

    generated = renderer.render_to_pdf(
        sections,
        out_pdf,
        v_scale_ratio=args.v_ratio,
        output_individual=args.individual,
    )

    print(f"Generated PDF: {out_pdf}")
    if args.individual:
        print(f"Generated {len(generated) - 1} individual PDFs in: {out_pdf.parent / '個別'}")


if __name__ == "__main__":
    main()
