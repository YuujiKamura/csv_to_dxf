import unittest
import pandas as pd
import os
import ezdxf
import loader
import dxf_draw_tenkaiz
import sys
import sys
import os

# プロジェクトルートへのパスを追加
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..')))


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



class TestDistanceHandling(unittest.TestCase):
    
    @classmethod
    def setUpClass(cls):
        # テスト実行前にヘッドレスモードを有効にする
        loader.set_headless_mode(True)
    
    @classmethod
    def tearDownClass(cls):
        # テスト終了後にヘッドレスモードを無効に戻す
        loader.set_headless_mode(False)
    
    def setUp(self):
        # 単延長と追加延長のテスト用ファイル名
        self.test_tanenkyo_file = 'test_tanenkyo.dxf'
        self.test_tsuikaenkyo_file = 'test_tsuikaenkyo.dxf'
    
    def tearDown(self):
        # テスト終了後にテスト用のDXFファイルを削除
        if os.path.exists(self.test_tanenkyo_file):
            os.remove(self.test_tanenkyo_file)
        if os.path.exists(self.test_tsuikaenkyo_file):
            os.remove(self.test_tsuikaenkyo_file)
    
    def test_tanenkyo_data(self):
        """単延長データの処理をテスト"""
        # 単延長データ（測点名、単延長、左幅員、右幅員）
        # 単延長: 各点の直前の点からの距離
        tanenkyo_data = pd.DataFrame({
            'name': ['No.0', 'No.1', 'No.2', 'No.3', 'No.4'],
            'x': [0.0, 10.0, 10.0, 15.0, 5.0],  # 単延長
            'wl': [3.0, 3.0, 3.5, 3.0, 3.0],
            'wr': [3.0, 3.0, 3.5, 3.0, 3.0]
        })
        
        # 追加延長に変換（起点からの累計距離に変換）
        tsuikaenkyo_data = tanenkyo_data.copy()
        tsuikaenkyo_data['x'] = tanenkyo_data['x'].cumsum()
        
        # データ内容を出力
        print("\n=============== 元のデータ ===============")
        print("\n単延長データ:")
        print(tanenkyo_data)
        
        print("\n追加延長に変換後:")
        print(tsuikaenkyo_data)
        
        # 単延長をそのまま使用してDXF生成（コード内で処理されるか確認）
        doc_tanenkyo = ezdxf.new(dxfversion="R2010")
        dxf_draw_tenkaiz.draw_road_sections(doc_tanenkyo.modelspace(), tanenkyo_data)
        doc_tanenkyo.saveas(self.test_tanenkyo_file)
        
        # 追加延長（累計距離）を使用してDXF生成（正しい処理）
        doc_tsuikaenkyo = ezdxf.new(dxfversion="R2010")
        dxf_draw_tenkaiz.draw_road_sections(doc_tsuikaenkyo.modelspace(), tsuikaenkyo_data)
        doc_tsuikaenkyo.saveas(self.test_tsuikaenkyo_file)
        
        # 両方のファイルが生成されることを確認
        self.assertTrue(os.path.exists(self.test_tanenkyo_file))
        self.assertTrue(os.path.exists(self.test_tsuikaenkyo_file))
        
        # 実際の処理の違いを確認するためにファイル情報を表示
        print(f"\n=============== ファイル生成結果 ===============")
        print(f"単延長ファイルが生成されました: {self.test_tanenkyo_file}")
        print(f"追加延長ファイルが生成されました: {self.test_tsuikaenkyo_file}")
        print("\n※注: dxf_draw_tenkaiz.pyではx座標に直接この値を使用するため、単延長の場合はレイアウトが異なります")
        
        # dxf_draw_tenkaiz.pyのcoodinate_dimensions関数の処理を確認
        print("\n=============== 距離計算の処理 ===============")
        row_tanenkyo = tanenkyo_data.iloc[1]  # No.1のデータ
        prev_points = ((0.0, 3.0), (0.0, 0.0), (0.0, -3.0))  # No.0の座標
        
        # 単延長の場合の寸法計算
        result_tanenkyo = dxf_draw_tenkaiz.coodinate_dimensions(row_tanenkyo, prev_points)
        
        # 追加延長の場合の寸法計算
        row_tsuikaenkyo = tsuikaenkyo_data.iloc[1]  # No.1のデータ
        result_tsuikaenkyo = dxf_draw_tenkaiz.coodinate_dimensions(row_tsuikaenkyo, prev_points)
        
        # 結果を表示
        print("\n== 単延長の場合の処理 ==")
        print(f"測点: {row_tanenkyo['name']}, 距離値: {row_tanenkyo['x']}")
        print(f"前の点からの計算された距離(tankyori): {row_tanenkyo['x'] - prev_points[1][0]}")
        
        print("\n== 追加延長の場合の処理 ==")
        print(f"測点: {row_tsuikaenkyo['name']}, 距離値: {row_tsuikaenkyo['x']}")
        print(f"前の点からの計算された距離(tankyori): {row_tsuikaenkyo['x'] - prev_points[1][0]}")
        
        # 寸法計算結果の詳細表示
        print("\n== 寸法条件の計算結果 ==")
        print("単延長での寸法条件:")
        for i, (condition, dim_info) in enumerate(result_tanenkyo):
            print(f"条件{i}: {condition}, 情報: {dim_info}")
        
        print("\n追加延長での寸法条件:")
        for i, (condition, dim_info) in enumerate(result_tsuikaenkyo):
            print(f"条件{i}: {condition}, 情報: {dim_info}")
        
        # 元の面積計算書データを適切な形式に変換する例を表示
        print("\n=============== 変換方法 ===============")
        print("元の面積計算書データを適切な形式に変換する方法:")
        print("1. CSVファイルを開いて必要な列（測点名、単延長、幅員）を抽出")
        print("2. 単延長を追加延長（累計距離）に変換するには、単延長列を累積和に変換")
        print("   例: df['追加延長'] = df['単延長'].cumsum()")
        print("3. 変換したデータで新しいCSVファイルを作成し、DXF変換に使用")
        
        # 区間1.csvを追加延長形式に変換する例
        print("\n=============== 区間1.csv の変換例 ===============")
        try:
            df = pd.read_csv('区間1.csv', header=None)
            print("区間1.csvの内容:")
            print(df)
            
            # 列名を追加
            df.columns = ['name', 'x', 'wl', 'wr']
            
            # 単延長から追加延長への変換例
            df_converted = df.copy()
            df_converted['x'] = df['x'].cumsum()
            
            print("\n追加延長に変換後:")
            print(df_converted)
            
            # 変換後のデータを保存
            df_converted.to_csv('区間1_追加延長.csv', index=False, header=False)
            print("\n変換後のデータを'区間1_追加延長.csv'に保存しました。")
        except Exception as e:
            print(f"区間1.csv の変換中にエラーが発生しました: {e}")

# 単体でスクリプトを実行した場合
if __name__ == '__main__':
    # unittest.main()をそのまま実行すると出力がキャプチャされるので、
    # testRunner=Noneを指定して出力をキャプチャしないようにする
    unittest.main(argv=[sys.argv[0], '-v'], exit=False) 