import unittest
import pandas as pd
import os
import ezdxf
import loader
import dxf_draw_tenkaiz
from table_to_dxf import main
import sys
import os
import sys
import os

# プロジェクトルートへのパスを追加
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..')))


# モジュールを条件付きでインポート
try:
    from src.table_to_dxf import main
import sys
import os
except ImportError:
    try:
        from table_to_dxf import main
import sys
import os
    except ImportError:
        print("警告: table_to_dxfモジュールが見つかりません。テストはスキップされます。")


# モジュールを条件付きでインポート
try:
    import src.ezdxf as ezdxf
except ImportError:
    try:
        import ezdxf
    except ImportError:
        print("警告: ezdxfモジュールが見つかりません。テストはスキップされます。")


# モジュールを条件付きでインポート
try:
    import src.loader as loader
except ImportError:
    try:
        import loader
    except ImportError:
        print("警告: loaderモジュールが見つかりません。テストはスキップされます。")


# モジュールを条件付きでインポート
try:
    import src.dxf_draw_tenkaiz as dxf_draw_tenkaiz
except ImportError:
    try:
        import dxf_draw_tenkaiz
    except ImportError:
        print("警告: dxf_draw_tenkaizモジュールが見つかりません。テストはスキップされます。")



)


# モジュールを条件付きでインポート
try:
    from src.table_to_dxf import main

class TestTableToDxf
except ImportError:
    try:
        from table_to_dxf import main

class TestTableToDxf
    except ImportError:
        print("警告: table_to_dxfモジュールが見つかりません。テストはスキップされます。")



class TestTableToDxf(unittest.TestCase):
    
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
        
        # テスト用の出力ファイル名
        self.test_dxf_path = 'test_road_sections.dxf'
        
    def tearDown(self):
        # テスト終了後にテスト用のDXFファイルを削除
        if os.path.exists(self.test_dxf_path):
            os.remove(self.test_dxf_path)
    
    def test_create_dxf_file(self):
        # DXFドキュメントの作成をテスト
        doc = ezdxf.new(dxfversion="R2010")
        self.assertIsNotNone(doc)
        
        # 単位設定をテスト
        doc.header['$INSUNITS'] = 6  # メートル単位
        doc.header['$MEASUREMENT'] = 1  # メトリック単位
        self.assertEqual(doc.header['$INSUNITS'], 6)
        self.assertEqual(doc.header['$MEASUREMENT'], 1)
        
    def test_draw_road_sections(self):
        # DXFドキュメントの作成
        doc = ezdxf.new(dxfversion="R2010")
        msp = doc.modelspace()
        
        # 道路断面の描画をテスト
        dxf_draw_tenkaiz.draw_road_sections(msp, self.test_data)
        
        # エンティティが作成されたことを確認
        entities = list(msp)
        self.assertGreater(len(entities), 0)
        
        # 保存して正常に完了することを確認
        doc.saveas(self.test_dxf_path)
        self.assertTrue(os.path.exists(self.test_dxf_path))
        
    def test_data_validation(self):
        # 正常なデータの検証
        valid_data = self.test_data.copy()
        result = loader.validate_data(valid_data)
        self.assertIsNotNone(result)
        
        # 列数が異なる不正なデータのテスト
        invalid_data = pd.DataFrame({
            'name': ['No.0', 'No.1'],
            'x': [0.0, 10.0],
            'wl': [3.45, 3.50]
        })
        result = loader.validate_data(invalid_data)
        self.assertIsNone(result)

if __name__ == '__main__':
    unittest.main() 