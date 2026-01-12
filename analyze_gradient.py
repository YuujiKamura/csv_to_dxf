#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
Excelファイル「計画.xlsx」から縦断勾配と横断勾配の最大最小値を解析
"""
import pandas as pd
import numpy as np
import sys
from pathlib import Path

# 標準出力のエンコーディングをUTF-8に設定
if sys.platform == 'win32':
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

def extract_gradient_values(df, keyword="勾配"):
    """データフレームから勾配値を抽出"""
    gradient_values = []
    
    # 数値データを抽出
    for col in df.columns:
        for idx in df.index:
            val = df.iloc[idx, col]
            if pd.notna(val):
                # 数値型か確認
                try:
                    num_val = float(val)
                    # 文字列型の列名や値で「勾配」を含む場合は除外
                    if isinstance(val, (int, float)) or (isinstance(val, str) and keyword not in str(val)):
                        # 合理的な範囲の数値のみ（-1から1の間、または-100から100の間など）
                        if -1000 <= num_val <= 1000:
                            gradient_values.append(num_val)
                except (ValueError, TypeError):
                    pass
    
    return gradient_values

def analyze_longitudinal_gradients(file_path: str):
    """縦断勾配を解析"""
    xls = pd.ExcelFile(file_path)
    
    results = {}
    
    # 縦断 (現況)
    if "縦断 (現況)" in xls.sheet_names:
        df = pd.read_excel(xls, sheet_name="縦断 (現況)", header=None)
        
        # 縦断勾配列を探す（「縦断勾配」という文字を含む列）
        gradient_cols = []
        gradient_values = []
        
        for col_idx in range(len(df.columns)):
            col_data = df.iloc[:, col_idx]
            # ヘッダー行を探す
            for row_idx in range(min(5, len(df))):
                cell_value = df.iloc[row_idx, col_idx]
                if pd.notna(cell_value) and "縦断勾配" in str(cell_value):
                    gradient_cols.append(col_idx)
                    break
        
        # 勾配値を抽出（ヘッダー行より下のデータ）
        for col_idx in gradient_cols:
            col_data = df.iloc[2:, col_idx]  # ヘッダー行をスキップ
            for val in col_data:
                if pd.notna(val):
                    try:
                        num_val = float(val)
                        # 合理的な範囲（-1～1、または-100～100など）
                        if -10 <= num_val <= 10:
                            gradient_values.append(num_val)
                    except (ValueError, TypeError):
                        pass
        
        if gradient_values:
            results["縦断 (現況)"] = {
                'values': gradient_values,
                'max': max(gradient_values),
                'min': min(gradient_values),
                'mean': np.mean(gradient_values),
                'count': len(gradient_values)
            }
    
    # 縦断 (計画)
    if "縦断 (計画)" in xls.sheet_names:
        df = pd.read_excel(xls, sheet_name="縦断 (計画)", header=None)
        
        gradient_cols = []
        gradient_values = []
        
        for col_idx in range(len(df.columns)):
            col_data = df.iloc[:, col_idx]
            for row_idx in range(min(5, len(df))):
                cell_value = df.iloc[row_idx, col_idx]
                if pd.notna(cell_value) and "縦断勾配" in str(cell_value):
                    gradient_cols.append(col_idx)
                    break
        
        for col_idx in gradient_cols:
            col_data = df.iloc[3:, col_idx]  # ヘッダー行をスキップ
            for val in col_data:
                if pd.notna(val):
                    try:
                        num_val = float(val)
                        if -10 <= num_val <= 10:
                            gradient_values.append(num_val)
                    except (ValueError, TypeError):
                        pass
        
        if gradient_values:
            results["縦断 (計画)"] = {
                'values': gradient_values,
                'max': max(gradient_values),
                'min': min(gradient_values),
                'mean': np.mean(gradient_values),
                'count': len(gradient_values)
            }
    
    return results

def analyze_cross_section_gradients(file_path: str):
    """横断勾配を解析"""
    xls = pd.ExcelFile(file_path)
    
    results = {}
    
    # 横断(現況)
    if "横断(現況)" in xls.sheet_names:
        df = pd.read_excel(xls, sheet_name="横断(現況)", header=None)
        
        gradient_values = []
        
        # 「勾配」という文字がある行を探す
        gradient_row_idx = None
        for row_idx in range(min(10, len(df))):
            for col_idx in range(len(df.columns)):
                cell_value = df.iloc[row_idx, col_idx]
                if pd.notna(cell_value) and "勾配" in str(cell_value) and "縦断" not in str(cell_value):
                    gradient_row_idx = row_idx
                    break
            if gradient_row_idx is not None:
                break
        
        if gradient_row_idx is not None:
            # 勾配行のデータを抽出
            gradient_row = df.iloc[gradient_row_idx + 1:, 2]  # 列2（勾配列）以降
            for val in gradient_row:
                if pd.notna(val):
                    try:
                        num_val = float(val)
                        # 横断勾配は通常-1～1の範囲
                        if -10 <= num_val <= 10:
                            gradient_values.append(num_val)
                    except (ValueError, TypeError):
                        pass
        
        if gradient_values:
            results["横断(現況)"] = {
                'values': gradient_values,
                'max': max(gradient_values),
                'min': min(gradient_values),
                'mean': np.mean(gradient_values),
                'count': len(gradient_values)
            }
    
    # 横断(計画)
    if "横断(計画)" in xls.sheet_names:
        df = pd.read_excel(xls, sheet_name="横断(計画)", header=None)
        
        gradient_values = []
        
        gradient_row_idx = None
        for row_idx in range(min(10, len(df))):
            for col_idx in range(len(df.columns)):
                cell_value = df.iloc[row_idx, col_idx]
                if pd.notna(cell_value) and "勾配" in str(cell_value) and "縦断" not in str(cell_value):
                    gradient_row_idx = row_idx
                    break
            if gradient_row_idx is not None:
                break
        
        if gradient_row_idx is not None:
            gradient_row = df.iloc[gradient_row_idx + 1:, 2]
            for val in gradient_row:
                if pd.notna(val):
                    try:
                        num_val = float(val)
                        if -10 <= num_val <= 10:
                            gradient_values.append(num_val)
                    except (ValueError, TypeError):
                        pass
        
        if gradient_values:
            results["横断(計画)"] = {
                'values': gradient_values,
                'max': max(gradient_values),
                'min': min(gradient_values),
                'mean': np.mean(gradient_values),
                'count': len(gradient_values)
            }
    
    return results

def main():
    file_path = "data/計画.xlsx"
    
    print("=" * 80)
    print("縦断勾配・横断勾配の最大最小値解析")
    print("=" * 80)
    
    # 縦断勾配の解析
    print("\n【縦断勾配】")
    print("-" * 80)
    longitudinal_results = analyze_longitudinal_gradients(file_path)
    
    for sheet_name, stats in longitudinal_results.items():
        print(f"\n{sheet_name}:")
        print(f"  データ数: {stats['count']}")
        print(f"  最大値: {stats['max']:.6f}")
        print(f"  最小値: {stats['min']:.6f}")
        print(f"  平均値: {stats['mean']:.6f}")
        print(f"  範囲: {stats['max'] - stats['min']:.6f}")
    
    # 横断勾配の解析
    print("\n\n【横断勾配】")
    print("-" * 80)
    cross_section_results = analyze_cross_section_gradients(file_path)
    
    for sheet_name, stats in cross_section_results.items():
        print(f"\n{sheet_name}:")
        print(f"  データ数: {stats['count']}")
        print(f"  最大値: {stats['max']:.6f}")
        print(f"  最小値: {stats['min']:.6f}")
        print(f"  平均値: {stats['mean']:.6f}")
        print(f"  範囲: {stats['max'] - stats['min']:.6f}")
    
    # 全体サマリー
    print("\n\n【全体サマリー】")
    print("-" * 80)
    
    all_longitudinal = []
    for stats in longitudinal_results.values():
        all_longitudinal.extend(stats['values'])
    
    all_cross_section = []
    for stats in cross_section_results.values():
        all_cross_section.extend(stats['values'])
    
    if all_longitudinal:
        print(f"\n縦断勾配（全体）:")
        print(f"  データ数: {len(all_longitudinal)}")
        print(f"  最大値: {max(all_longitudinal):.6f}")
        print(f"  最小値: {min(all_longitudinal):.6f}")
        print(f"  平均値: {np.mean(all_longitudinal):.6f}")
        print(f"  範囲: {max(all_longitudinal) - min(all_longitudinal):.6f}")
    
    if all_cross_section:
        print(f"\n横断勾配（全体）:")
        print(f"  データ数: {len(all_cross_section)}")
        print(f"  最大値: {max(all_cross_section):.6f}")
        print(f"  最小値: {min(all_cross_section):.6f}")
        print(f"  平均値: {np.mean(all_cross_section):.6f}")
        print(f"  範囲: {max(all_cross_section) - min(all_cross_section):.6f}")
    
    print("\n" + "=" * 80)

if __name__ == "__main__":
    main()
