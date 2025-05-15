#!/usr/bin/env python
# -*- coding: utf-8 -*-

import pandas as pd
import sys
import os

# モジュールへのパスを追加
sys.path.append(os.path.abspath(os.path.dirname(__file__)))
from src.station_name_utils import preserve_and_complete_station_names
from src.processing import transform_section

def test_with_existing_names():
    """テストケース1 - 既存の測点名がある場合（区間3のCSVデータ）"""
    # 区間3のような実データを模したデータフレーム（既にNo.プレフィックスがある）
    data = {
        'name': ['0+7.9', 'No.0+9.9', 'No.0+11.5', 'No.0+14', 'No.0+16.1'],
        'x': [0, 2.03, 3.63, 6.13, 8.23]  # これは累積距離
    }
    df = pd.DataFrame(data)
    
    print("===== テストケース1: 既存の測点名がある場合 =====")
    print("入力データ:")
    print(df)
    
    # 通常の補完処理（既存の測点名を保持）
    result1 = preserve_and_complete_station_names(df, keep_existing=True)
    print("\n結果1（keep_existing=True）:")
    print(result1)
    
    # 強制的に再計算
    result2 = preserve_and_complete_station_names(df, keep_existing=False)
    print("\n結果2（keep_existing=False）:")
    print(result2)
    
    # transform_section関数を使用した場合
    result3 = transform_section(df, 3)
    print("\n結果3（transform_section）:")
    print(result3)

def test_with_empty_names():
    """テストケース2 - 最初の測点名以外が空の場合"""
    # 区間3のような実データを模したデータフレーム
    data = {
        'name': ['0+7.9', '', '', '', ''],
        'x': [0, 2.03, 3.63, 6.13, 8.23]  # これは累積距離
    }
    df = pd.DataFrame(data)
    
    print("\n===== テストケース2: 最初の測点名以外が空の場合 =====")
    print("入力データ:")
    print(df)
    
    # 測点名を生成して補完
    result = preserve_and_complete_station_names(df)
    
    print("\n処理結果:")
    print(result)
    
    # 期待される結果
    expected_names = ['0+7.9', '0+9.9', '0+11.5', '0+14', '0+16.1']
    
    print("\n検証:")
    for i, (actual, expected) in enumerate(zip(result['name'], expected_names)):
        print(f"行 {i}: 実際={actual}, 期待={expected}, {'一致' if actual == expected else '不一致'}")

def main():
    test_with_existing_names()
    test_with_empty_names()

if __name__ == "__main__":
    main() 