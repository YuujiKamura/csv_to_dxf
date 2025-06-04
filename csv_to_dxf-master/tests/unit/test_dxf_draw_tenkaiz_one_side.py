import unittest
import pandas as pd
import sys
import os

# プロジェクトルートへのパスを追加
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..')))

# モジュールを条件付きでインポート
try:
    import src.ezdxf as ezdxf
    from src.ezdxf.enums import TextEntityAlignment
except ImportError:
    try:
        import ezdxf
        from ezdxf.enums import TextEntityAlignment
    except ImportError:
        print("警告: ezdxfモジュールが見つかりません。テストはスキップされます。")

# モジュールを条件付きでインポート
try:
    import src.dxf_draw_tenkaiz as dxf_draw_tenkaiz
except ImportError:
    try:
        import dxf_draw_tenkaiz
    except ImportError:
        print("警告: dxf_draw_tenkaizモジュールが見つかりません。テストはスキップされます。")

# モジュールを条件付きでインポート
try:
    import src.loader as loader
except ImportError:
    try:
        import loader
    except ImportError:
        print("警告: loaderモジュールが見つかりません。テストはスキップされます。")

class TestDxfDrawTenkaizOneSide(unittest.TestCase):
    
    @classmethod
    def setUpClass(cls):
        # テスト実行前にヘッドレスモードを有効にする
        loader.set_headless_mode(True)
    
    @classmethod
    def tearDownClass(cls):
        # テスト終了後にヘッドレスモードを無効に戻す
        loader.set_headless_mode(False)
    
    def setUp(self):
        # テスト用のサンプルデータを作成（左側幅員のみ）
        self.test_data_left_only = pd.DataFrame({
            'name': ['No.0', 'No.1', 'No.2'],
            'x': [0.0, 10.0, 20.0],
            'wl': [3.45, 3.50, 3.45],
            'wr': [0.0, 0.0, 0.0]  # 右側幅員は0
        })
        
        # テスト用のサンプルデータを作成（右側幅員のみ）
        self.test_data_right_only = pd.DataFrame({
            'name': ['No.0', 'No.1', 'No.2'],
            'x': [0.0, 10.0, 20.0],
            'wl': [0.0, 0.0, 0.0],  # 左側幅員は0
            'wr': [3.55, 3.50, 3.55]
        })
        
        # テスト用のDXFドキュメントを作成
        self.doc = ezdxf.new(dxfversion="R2010")
        self.msp = self.doc.modelspace()
        
        # テスト用の出力ファイル名
        self.test_dxf_path_left = 'test_road_sections_left_only.dxf'
        self.test_dxf_path_right = 'test_road_sections_right_only.dxf'
    
    def tearDown(self):
        # テスト終了後にテスト用のDXFファイルを削除
        if os.path.exists(self.test_dxf_path_left):
            os.remove(self.test_dxf_path_left)
        if os.path.exists(self.test_dxf_path_right):
            os.remove(self.test_dxf_path_right)
    
    def test_draw_road_sections_left_only(self):
        # 左側幅員のみのデータで道路断面の描画をテスト
        doc_left = ezdxf.new(dxfversion="R2010")
        msp_left = doc_left.modelspace()
        
        # 道路断面の描画
        dxf_draw_tenkaiz.draw_road_sections(msp_left, self.test_data_left_only)
        
        # エンティティが作成されたことを確認
        entities = list(msp_left)
        self.assertGreater(len(entities), 0)
        
        # ラインとテキストのエンティティをカウント
        lines = [e for e in entities if e.dxftype() == 'LINE']
        texts = [e for e in entities if e.dxftype() == 'TEXT']
        self.assertGreater(len(lines), 0)
        self.assertGreater(len(texts), 0)
        
        # 保存して正常に完了することを確認
        doc_left.saveas(self.test_dxf_path_left)
        self.assertTrue(os.path.exists(self.test_dxf_path_left))
    
    def test_draw_road_sections_right_only(self):
        # 右側幅員のみのデータで道路断面の描画をテスト
        doc_right = ezdxf.new(dxfversion="R2010")
        msp_right = doc_right.modelspace()
        
        # 道路断面の描画
        dxf_draw_tenkaiz.draw_road_sections(msp_right, self.test_data_right_only)
        
        # エンティティが作成されたことを確認
        entities = list(msp_right)
        self.assertGreater(len(entities), 0)
        
        # ラインとテキストのエンティティをカウント
        lines = [e for e in entities if e.dxftype() == 'LINE']
        texts = [e for e in entities if e.dxftype() == 'TEXT']
        self.assertGreater(len(lines), 0)
        self.assertGreater(len(texts), 0)
        
        # 保存して正常に完了することを確認
        doc_right.saveas(self.test_dxf_path_right)
        self.assertTrue(os.path.exists(self.test_dxf_path_right))
    
    def test_coordinate_dimensions_left_only(self):
        # 左側幅員のみの行データ
        row = pd.Series({
            'name': 'No.1',
            'x': 10.0,
            'wl': 3.50,
            'wr': 0.0  # 右側幅員は0
        })
        prev_points = ((0.0, 3.45), (0.0, 0), (0.0, 0.0))  # 前のポイント（右側幅員0）
        
        # 寸法座標計算をテスト
        result = dxf_draw_tenkaiz.coordinate_dimensions(row, prev_points)
        
        # 結果をチェック
        # 左側幅員の寸法が含まれていることを確認
        left_width_dimensions = [dim for cond, dim in result if cond and dim[0] == "3.50"]
        self.assertGreater(len(left_width_dimensions), 0)
        
        # 右側幅員の寸法が含まれていないことを確認（wr=0.0のため）
        right_width_dimensions = [dim for cond, dim in result if cond and dim[0] == "0.00"]
        self.assertEqual(len(right_width_dimensions), 0)
    
    def test_coordinate_dimensions_right_only(self):
        # 右側幅員のみの行データ
        row = pd.Series({
            'name': 'No.1',
            'x': 10.0,
            'wl': 0.0,  # 左側幅員は0
            'wr': 3.50
        })
        prev_points = ((0.0, 0.0), (0.0, 0), (0.0, -3.55))  # 前のポイント（左側幅員0）
        
        # 寸法座標計算をテスト
        result = dxf_draw_tenkaiz.coordinate_dimensions(row, prev_points)
        
        # 結果をチェック
        # 左側幅員の寸法が含まれていないことを確認（wl=0.0のため）
        left_width_dimensions = [dim for cond, dim in result if cond and dim[0] == "0.00"]
        self.assertEqual(len(left_width_dimensions), 0)
        
        # 右側幅員の寸法が含まれていることを確認
        right_width_dimensions = [dim for cond, dim in result if cond and dim[0] == "3.50"]
        self.assertGreater(len(right_width_dimensions), 0)
    
    def test_validate_data_one_side(self):
        # 片側幅員データもバリデーションを通過することを確認
        result_left = loader.validate_data(self.test_data_left_only)
        self.assertIsNotNone(result_left)
        
        result_right = loader.validate_data(self.test_data_right_only)
        self.assertIsNotNone(result_right)

if __name__ == '__main__':
    unittest.main() 