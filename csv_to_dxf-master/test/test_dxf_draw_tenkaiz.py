import unittest
import pandas as pd
import ezdxf
from ezdxf.enums import TextEntityAlignment
import dxf_draw_tenkaiz
import loader

class TestDxfDrawTenkaiz(unittest.TestCase):
    
    @classmethod
    def setUpClass(cls):
        # テスト実行前にヘッドレスモードを有効にする
        loader.set_headless_mode(True)
    
    @classmethod
    def tearDownClass(cls):
        # テスト終了後にヘッドレスモードを無効に戻す
        loader.set_headless_mode(False)
    
    def setUp(self):
        # テスト用のサンプルデータを作成
        self.test_data = pd.DataFrame({
            'name': ['No.0', 'No.1', 'No.2'],
            'x': [0.0, 10.0, 20.0],
            'wl': [3.45, 3.50, 3.45],
            'wr': [3.55, 3.50, 3.55]
        })
        
        # テスト用のDXFドキュメントを作成
        self.doc = ezdxf.new(dxfversion="R2010")
        self.msp = self.doc.modelspace()
    
    def test_draw_road_sections(self):
        # 道路断面の描画をテスト
        dxf_draw_tenkaiz.draw_road_sections(self.msp, self.test_data)
        
        # エンティティが作成されたことを確認
        entities = list(self.msp)
        self.assertGreater(len(entities), 0)
        
        # ラインとテキストのエンティティが作成されたことを確認
        lines = [e for e in entities if e.dxftype() == 'LINE']
        texts = [e for e in entities if e.dxftype() == 'TEXT']
        self.assertGreater(len(lines), 0)
        self.assertGreater(len(texts), 0)
    
    def test_coodinate_lines(self):
        # 行データ - DataFrameのSeriesオブジェクトとして作成
        row = pd.Series({
            'name': 'No.1',
            'x': 10.0,
            'wl': 3.50,
            'wr': 3.50
        })
        prev_linelr = ((0.0, 3.45), (0.0, 0), (0.0, -3.55))  # No.0のデータから計算される座標
        
        # 座標計算をテスト
        result = dxf_draw_tenkaiz.coodinate_lines(row, prev_linelr)
        
        # 結果は5つの条件と座標のタプルのリストであることを確認
        self.assertEqual(len(result), 5)
        
        # 各条件と座標をチェック
        for condition, points in result:
            # 条件はブール値で、座標は2つの点のタプルであることを確認
            self.assertIsInstance(condition, bool)
            self.assertEqual(len(points), 2)
            for point in points:
                self.assertEqual(len(point), 2)
    
    def test_coodinate_dimensions(self):
        # 行データ - DataFrameのSeriesオブジェクトとして作成
        row = pd.Series({
            'name': 'No.1',
            'x': 10.0,
            'wl': 3.50,
            'wr': 3.50
        })
        prev_points = ((0.0, 3.45), (0.0, 0), (0.0, -3.55))  # No.0のデータから計算される座標
        
        # 寸法座標計算をテスト
        result = dxf_draw_tenkaiz.coodinate_dimensions(row, prev_points)
        
        # 結果は4つの条件と寸法情報のタプルのリストであることを確認
        self.assertEqual(len(result), 4)
        
        # 各条件と寸法情報をチェック
        for condition, dim_info in result:
            # 条件はブール値で、寸法情報は4つの要素を持つタプルであることを確認
            self.assertIsInstance(condition, bool)
            self.assertEqual(len(dim_info), 4)
            
            # テキスト、位置、回転、配置をチェック
            text, position, rotation, alignment = dim_info
            self.assertIsInstance(text, str)
            self.assertEqual(len(position), 2)
            self.assertIsInstance(rotation, (int, float))
            self.assertIn(alignment, [TextEntityAlignment.TOP_CENTER, TextEntityAlignment.BOTTOM_CENTER])
    
    def test_align_by_distance(self):
        # 短い距離の場合はBOTTOM_CENTERを返す
        self.assertEqual(dxf_draw_tenkaiz.align_by_distance(0.5), TextEntityAlignment.BOTTOM_CENTER)
        
        # 長い距離の場合はTOP_CENTERを返す
        self.assertEqual(dxf_draw_tenkaiz.align_by_distance(1.0), TextEntityAlignment.TOP_CENTER)
        self.assertEqual(dxf_draw_tenkaiz.align_by_distance(10.0), TextEntityAlignment.TOP_CENTER)

if __name__ == '__main__':
    unittest.main() 