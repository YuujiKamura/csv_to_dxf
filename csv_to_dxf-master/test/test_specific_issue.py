#!/usr/bin/env python
# -*- coding: utf-8 -*-

import pandas as pd
import sys
import os
from pathlib import Path

# モジュールへのパスを追加
sys.path.append(os.path.abspath(os.path.dirname(__file__)))
from src.processing import extract_section_data, transform_section
from src.station_name_utils import preserve_and_complete_station_names

def create_test_data_for_section5():
    """区間5のテスト用データを作成"""
    # 区間5のテスト用データ
    data = {
        'name': ['7+13.7', 'No.8', '', '', '', 'No.9', '9+5.7'],
        'x': [0.00, 6.30, 11.75, 2.05, 2.05, 2.10, 5.70]
    }
    return pd.DataFrame(data)

def test_important_cases():
    """重要なケースを個別にテスト"""
    print("\n===== 主要ケースのテスト =====")
    
    # ケース1: 「7+20」が「No.8」になるかどうか
    data1 = {
        'name': ['No.7', ''],
        'x': [0.00, 20.00]  # No.7から20m
    }
    df1 = pd.DataFrame(data1)
    
    print("\nケース1: 「7+20」が「No.8」になるかどうか")
    print("入力:")
    print(df1)
    
    # 測点名の自動補完を実行
    result1 = preserve_and_complete_station_names(df1, keep_existing=False)
    print("\n結果:")
    print(result1)
    print(f"PASS: {result1['name'][1] == 'No.8'}")
    
    # ケース2: 区間5のデータで「7+20」が「No.8」になるかどうか
    df2 = create_test_data_for_section5()
    
    # 累積距離に変換
    df2['x'] = df2['x'].cumsum()
    
    print("\nケース2: 区間5のデータ")
    print("入力:")
    print(df2)
    
    # 測点名の自動補完を実行
    result2 = preserve_and_complete_station_names(df2, keep_existing=False)
    print("\n結果:")
    print(result2)
    
    # 詳細情報の表示
    print("\n測点の詳細情報:")
    for idx, row in result2.iterrows():
        print(f"行 {idx}: {row['name']} = {row['x']}")
    
    # 検証: No.8位置の正しさ
    eight_index = result2.index[result2['name'] == 'No.8'][0] if 'No.8' in result2['name'].values else None
    if eight_index is not None:
        eight_position = result2.at[eight_index, 'x']
        print(f"No.8の位置: {eight_position}")
        # No.8は入力データで明示的に6.3に設定されているので、それを期待値とする
        print(f"PASS: No.8の位置が正しい（期待値≈6.30）: {abs(eight_position - 6.3) < 0.1}")
    else:
        print("FAIL: No.8が見つかりません")

def main():
    """特定の問題を検証するためのテスト"""
    test_important_cases()

if __name__ == "__main__":
    main() 