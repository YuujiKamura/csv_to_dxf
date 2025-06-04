import unittest
import os
import sys
import pandas as pd
import tempfile
import shutil

# プロジェクトルートへのパスを追加
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..')))

# モジュールを条件付きでインポート
try:
    from gui.core.csv_processor import CSVProcessor
except ImportError:
    try:
        from src.gui.core.csv_processor import CSVProcessor
    except ImportError:
        print("警告: csv_processorモジュールが見つかりません。テストはスキップされます。")
        CSVProcessor = None

# テスト対象のモジュールへのパスを追加
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

class TestCSVProcessor(unittest.TestCase):
    """CSVProcessorクラスのテスト"""
    
    def setUp(self):
        """テスト前の準備"""
        if CSVProcessor is None:
            self.skipTest("CSVProcessorモジュールがインポートできませんでした")
        self.processor = CSVProcessor()
        # テスト用のファイルパスを修正
        self.test_csv = os.path.abspath(os.path.join(os.path.dirname(__file__), '../おやま.csv'))
    
    def tearDown(self):
        """テスト後のクリーンアップ"""
        # 何もしない（ファイルは削除しない）
        pass
    
    def test_read_csv_file(self):
        """ファイル読み込みのテスト"""
        # 存在するファイルを読み込む
        result = self.processor.read_csv_file(self.test_csv)
        self.assertTrue(result)
        self.assertEqual(str(self.processor.csv_path), self.test_csv)
        
        # 存在しないファイルを読み込むとFalseが返る
        non_existent = "non_existent_file.csv"
        result = self.processor.read_csv_file(non_existent)
        self.assertFalse(result)
    
    def test_process_section_data(self):
        """区間データ処理のテスト"""
        # ファイルを読み込む
        self.processor.read_csv_file(self.test_csv)
        
        # 単一区間のデータを処理（CSVには区間指定がないため、デフォルトとして処理される）
        result = self.processor.process_section_data("")
        
        # 結果を検証
        self.assertIsInstance(result, pd.DataFrame)
        self.assertEqual(len(result), 4)  # 4行のデータ
        
        # 幅員列の存在を確認
        self.assertTrue('wl' in result.columns)
        self.assertTrue('wr' in result.columns)
    
    def test_save_csv(self):
        """CSVの保存テスト"""
        # ファイルを読み込み、処理する
        self.processor.read_csv_file(self.test_csv)
        processed_data = self.processor.process_section_data("")
        
        # 一時ディレクトリに保存
        temp_dir = tempfile.mkdtemp()
        
        try:
            # 保存を実行
            result = self.processor.save_csv("", temp_dir)
            
            # 結果を検証
            self.assertTrue(result)
            output_path = os.path.join(temp_dir, ".csv")
            self.assertTrue(os.path.exists(output_path))
            
            # 保存されたファイルを読み込んで検証
            saved_df = pd.read_csv(output_path)
            self.assertEqual(len(saved_df), 4)
        finally:
            # 出力ディレクトリを削除
            shutil.rmtree(temp_dir)

    def test_cache_cleared_on_width_assign_change(self):
        """幅員割当てを切り替えたときにキャッシュがクリアされるかを確認"""
        self.processor.read_csv_file(self.test_csv)
        
        # CSVファイルには区間が含まれていないので空の文字列を使用
        section = ""
        
        # まず左幅員(wl)で処理
        df_left = self.processor.process_section_data(section, width_assign="wl")
        self.assertIsNotNone(df_left)
        
        # 次に右幅員(wr)で処理
        df_right = self.processor.process_section_data(section, width_assign="wr")
        self.assertIsNotNone(df_right)
        
        # 幅員列の値が異なることを確認
        self.assertFalse(df_left["wl"].equals(df_right["wl"]))
        self.assertFalse(df_left["wr"].equals(df_right["wr"]))

if __name__ == '__main__':
    unittest.main() 