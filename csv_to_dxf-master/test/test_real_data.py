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

def test_section(csv_path, section_name, section_no):
    """指定された区間のデータを処理してテスト"""
    print(f"\n===== {csv_path} から {section_name} のデータを抽出して処理 =====")
    
    # CSV内の区間データを取得
    raw_data = extract_section_data(csv_path, section_name)
    
    if raw_data is None or raw_data.empty:
        print(f"エラー: {section_name} のデータが見つかりません")
        return
    
    print("\n1. 抽出されたデータ:")
    print(raw_data)
    
    # 変換処理を実行
    transformed_data = transform_section(raw_data, section_no)
    
    print("\n2. 処理結果 (transform_section):")
    print(transformed_data)
    
    # 詳細な検証
    print("\n3. 詳細な測点名チェック:")
    for i, row in transformed_data.iterrows():
        x = row['x']
        name = row['name']
        if '+' in name:
            # 補間測点は「No.」プレフィックスなし
            if name.startswith('No.'):
                print(f"  行 {i}: x={x}, 名前={name} - エラー: 補間測点に「No.」プレフィックスがあります")
            else:
                print(f"  行 {i}: x={x}, 名前={name} - OK: 補間測点に「No.」プレフィックスなし")
        else:
            # 主測点は「No.」プレフィックスあり
            if not name.startswith('No.'):
                print(f"  行 {i}: x={x}, 名前={name} - エラー: 主測点に「No.」プレフィックスがありません")
            else:
                print(f"  行 {i}: x={x}, 名前={name} - OK: 主測点に「No.」プレフィックス付き")

def main():
    """実データを使ったテスト - 複数区間のデータを処理"""
    csv_path = Path("data/面積計算書、塩塚小野線側溝改修工事 - シート1.csv")
    
    # 区間3のテスト
    test_section(csv_path, "区間3", 3)
    
    # 区間5のテスト
    test_section(csv_path, "区間5", 5)

if __name__ == "__main__":
    main() 