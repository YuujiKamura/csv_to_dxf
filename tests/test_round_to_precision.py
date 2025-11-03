import unittest
import pandas as pd
import sys
import os

# テスト対象のモジュールへのパスを追加
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))
from src.station_name_utils import round_to_precision

class TestRoundToPrecision(unittest.TestCase):
    """桁の丸め関数のテスト"""
    
    def test_round_to_precision_basic(self):
        """基本的な丸め処理のテスト"""
        # 単一の値のテスト
        self.assertEqual(round_to_precision(1.234, 0.01), 1.23)
        self.assertEqual(round_to_precision(1.235, 0.01), 1.24)
        self.assertEqual(round_to_precision(1.236, 0.01), 1.24)
        
        # 特殊ケースのテスト
        self.assertEqual(round_to_precision(0, 0.01), 0)
        self.assertEqual(round_to_precision(0.005, 0.01), 0.01)
        self.assertEqual(round_to_precision(0.004, 0.01), 0)
    
    def test_round_to_precision_different_precision(self):
        """異なる精度でのテスト"""
        # 0.01精度
        self.assertEqual(round_to_precision(0.007, 0.01), 0.01)
        self.assertEqual(round_to_precision(0.003, 0.01), 0)
        self.assertEqual(round_to_precision(0.01499, 0.01), 0.01)
        self.assertEqual(round_to_precision(0.01501, 0.01), 0.02)
        
        # 0.001精度（3桁）
        self.assertEqual(round_to_precision(0.0007, 0.001), 0.001)
        self.assertEqual(round_to_precision(0.0003, 0.001), 0)
        self.assertEqual(round_to_precision(0.0015, 0.001), 0.002)
        self.assertEqual(round_to_precision(0.0005, 0.001), 0.001)
        
        # 0.1精度
        self.assertEqual(round_to_precision(1.23, 0.1), 1.2)
        self.assertEqual(round_to_precision(1.25, 0.1), 1.3)
        self.assertEqual(round_to_precision(1.24, 0.1), 1.2)
    
    def test_round_to_precision_series(self):
        """Pandas Series型の値のテスト"""
        # シリーズのテスト
        series = pd.Series([1.234, 1.235, 1.236, 0, 0.005, 0.004])
        expected = pd.Series([1.23, 1.24, 1.24, 0, 0.01, 0])
        pd.testing.assert_series_equal(round_to_precision(series, 0.01).reset_index(drop=True), 
                                     expected.reset_index(drop=True))
    
    def test_round_to_precision_edge_cases(self):
        """エッジケースのテスト"""
        # 小さな値のテスト
        self.assertEqual(round_to_precision(0.001, 0.01), 0)
        self.assertEqual(round_to_precision(0.009, 0.01), 0.01)
        
        # 境界値テスト
        self.assertEqual(round_to_precision(0.015, 0.01), 0.02)
        self.assertEqual(round_to_precision(0.014, 0.01), 0.01)
        
        # 負の値のテスト
        self.assertEqual(round_to_precision(-0.005, 0.01), -0.01)
        self.assertEqual(round_to_precision(-0.004, 0.01), 0)
        self.assertEqual(round_to_precision(-1.235, 0.01), -1.24)
    
    def test_round_to_precision_special_cases(self):
        """特殊なケースのテスト"""
        # 小数点以下の桁数が多い場合
        self.assertEqual(round_to_precision(1.2345678, 0.01), 1.23)
        self.assertEqual(round_to_precision(1.2356789, 0.01), 1.24)
        
        # 大きな整数値
        self.assertEqual(round_to_precision(1000.005, 0.01), 1000.01)
        self.assertEqual(round_to_precision(9999.994, 0.01), 9999.99)
        
        # 非常に小さな精度
        self.assertEqual(round_to_precision(0.0001234, 0.0001), 0.0001)
        self.assertEqual(round_to_precision(0.0001567, 0.0001), 0.0002)

if __name__ == '__main__':
    unittest.main() 