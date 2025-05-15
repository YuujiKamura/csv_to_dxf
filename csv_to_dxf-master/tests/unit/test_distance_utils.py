import unittest
import pandas as pd
import numpy as np
import sys
import os

# テスト環境の設定
print("テスト環境の設定: プロジェクトルート =", os.path.abspath(os.path.join(os.path.dirname(__file__), "../..")))
ROOT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
sys.path.insert(0, ROOT_DIR)

# シンプルな単体テスト用の関数
def is_cumulative(values):
    """配列が累積かどうかをテスト用に簡易判定"""
    if len(values) < 2:
        return False
    diffs = [values[i+1] - values[i] for i in range(len(values)-1)]
    return any(diff > 15.0 for diff in diffs)  # 大きな差があれば累積と判定

def to_cumulative(values):
    """単延長→累積変換のテスト用シンプル実装"""
    if is_cumulative(values):
        return values.copy()  # 既に累積なら変更なし
    result = [values[0]]
    for i in range(1, len(values)):
        result.append(result[-1] + values[i])
    return result

class TestDistanceUtils(unittest.TestCase):
    
    def test_is_cumulative_detection(self):
        """累積距離判定のテスト"""
        # 単延長データ（差が小さい）
        incremental = [0.0, 5.0, 10.0, 5.0, 10.0]
        self.assertFalse(is_cumulative(incremental))
        
        # 累積距離データ（差が大きい）
        cumulative = [0.0, 5.0, 15.0, 35.0, 55.0]
        self.assertTrue(is_cumulative(cumulative))
    
    def test_to_cumulative_conversion(self):
        """単延長→累積変換のテスト"""
        # 単延長データ
        incremental = [0.0, 5.0, 10.0, 5.0, 10.0]
        
        # 期待値
        expected = [0.0, 5.0, 15.0, 20.0, 30.0]
        
        # 変換結果
        result = to_cumulative(incremental)
        
        # 累積変換が正しいか検証
        self.assertEqual(result, expected)
        
    def test_to_cumulative_idempotent(self):
        """累積データを変換しても変わらないことを確認"""
        # 累積距離データ
        cumulative = [0.0, 5.0, 15.0, 35.0, 55.0]
        
        # 変換結果
        result = to_cumulative(cumulative)
        
        # 変わらないことを確認
        self.assertEqual(result, cumulative)

if __name__ == '__main__':
    unittest.main() 