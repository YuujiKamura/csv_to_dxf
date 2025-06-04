import unittest
import os
import sys
import pandas as pd
import re

# テスト対象のモジュールへのパスを追加
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))
from gui.core.csv_processor import CSVProcessor

class TestE2ECSVProcessor(unittest.TestCase):
    """CSVProcessorクラスの実データを使ったE2Eテスト"""
    
    def setUp(self):
        """テスト前の準備"""
        self.processor = CSVProcessor()
        
        # 実際のデータファイルのパス
        self.real_csv_multiple = os.path.join(
            os.path.dirname(os.path.dirname(__file__)), 
            'data', 
            '面積計算書、塩塚小野線側溝改修工事 - シート1.csv'
        )
        self.real_csv_single = os.path.join(
            os.path.dirname(os.path.dirname(__file__)), 
            'data', 
            'data.csv'
        )
    
    def test_read_real_csv_multiple_sections(self):
        """実際の複数区間CSVファイルの読み込みテスト"""
        # 実在するファイルを読み込む
        result = self.processor.read_csv_file(self.real_csv_multiple)
        self.assertTrue(result)
        self.assertEqual(str(self.processor.file_path), os.path.abspath(self.real_csv_multiple))
        self.assertIsNone(self.processor.csv_data)
    
    def test_get_real_sections(self):
        """実際のCSVファイルの区間取得テスト"""
        # ファイルを読み込む
        self.processor.read_csv_file(self.real_csv_multiple)
        
        # 区間を取得
        sections = self.processor.get_available_sections()
        
        # 結果を検証
        self.assertIsInstance(sections, list)
        self.assertGreaterEqual(len(sections), 4)  # 少なくとも4つの区間があるはず
        self.assertIn('区間1', sections)
        self.assertIn('区間3', sections)
        self.assertIn('区間5', sections)
        self.assertIn('区間6', sections)
    
    def test_extract_section1_data(self):
        """区間1データ抽出のテスト"""
        # ファイルを読み込む
        self.processor.read_csv_file(self.real_csv_multiple)
        
        # 区間1のデータを抽出
        result = self.processor.extract_section_data('区間1')
        
        # 結果を検証
        self.assertIsInstance(result, pd.DataFrame)
        self.assertGreaterEqual(len(result), 5)  # 少なくとも5行のデータ
        self.assertTrue('name' in result.columns)
        self.assertTrue('x' in result.columns)
        self.assertTrue('wl' in result.columns)
        self.assertTrue('wr' in result.columns)
        self.assertIs(self.processor.csv_data, result)
    
    def test_extract_section3_data(self):
        """区間3データ抽出のテスト"""
        # ファイルを読み込む
        self.processor.read_csv_file(self.real_csv_multiple)
        
        # 区間3のデータを抽出
        result = self.processor.extract_section_data('区間3')
        
        # 結果を検証
        self.assertIsInstance(result, pd.DataFrame)
        self.assertGreaterEqual(len(result), 20)  # 区間3は行数が多い
        self.assertTrue('name' in result.columns)
        self.assertTrue('x' in result.columns)
        self.assertIs(self.processor.csv_data, result)
    
    def test_process_section_normalize_names(self):
        """測点名正規化テスト - すべての区間で測点名が正規化されること"""
        # ファイルを読み込む
        self.processor.read_csv_file(self.real_csv_multiple)
        
        # 各区間の期待される測点名（key: 区間名, value: 期待される測点名のリスト）
        expected_names = {
            '区間1': ['No.0', '0+1.2', '0+3.3', '0+5.4', '0+7'],
            '区間3': ['0+7.9', '0+9.9', '0+11.5', '0+14', '0+16.1', 'No.1', 'No.2', 'No.3',
                    '3+9.6', '3+11.6', '3+13.7', '3+15.7', '3+17.8', 'No.4',
                    '4+1', '4+3', '4+5', '4+7', '4+9.1', '4+11.1',
                    '4+13.2', '4+15.2', '4+17.3', 'No.5', '5+1.5', '5+3.6',
                    '5+5.6', '5+13', 'No.6', 'No.7', '7+12.8'],
            '区間5': ['7+13.7', 'No.8', '8+11.8', '8+13.8', '8+15.8', '8+17.9', 'No.9', '9+5.7'],
            '区間6': ['7+13.7', 'No.8', '8+12.5', '8+14', '8+15.7', '8+17.3', 'No.9', '9+5.7',
                    '9+5.7', '9+8', '10+1.2']
        }
        
        # テストする区間
        sections_to_test = ['区間1', '区間3', '区間5', '区間6']
        
        for section_name in sections_to_test:
            with self.subTest(section=section_name):
                # 区間データを処理（測点名の正規化を有効に）
                result = self.processor.process_section_data(
                    section_name,
                    auto_convert=True,
                    normalize_station_names=True
                )

                # 結果を検証
                self.assertIsInstance(result, pd.DataFrame)
                self.assertGreater(len(result), 0)
                self.assertIs(self.processor.csv_data, result)
                
                # 区間の詳細データを出力
                print(f"\n===== {section_name}の処理後データ =====")
                for i, row in result.iterrows():
                    print(f"行 {i}: 名前='{row['name']}', x={row.get('x', 'N/A')}")
                print("============================\n")
                
                # 期待される測点名と一致するか確認
                expected = expected_names[section_name]
                actual = result['name'].tolist()
                
                # 長さが同じか確認
                self.assertEqual(len(expected), len(actual), 
                                f"{section_name}の測点名の数が期待値と異なります。期待={len(expected)}, 実際={len(actual)}")
                
                # 各測点名が期待値と一致するか確認
                for i, (exp, act) in enumerate(zip(expected, actual)):
                    self.assertEqual(exp, act, 
                                  f"{section_name}の{i+1}番目の測点名が期待値と異なります。期待='{exp}', 実際='{act}'")
                
                # すべての測点名が有効な形式（No.X形式またはX+Y形式または単純な数値）になっているか確認
                for name in result['name']:
                    self.assertTrue(
                        bool(re.match(r'No\.\d+(\+\d+)?', str(name))) or  # No.X形式
                        bool(re.match(r'\d+\+\d+(\.\d+)?', str(name))) or # X+Y形式
                        bool(re.match(r'\d+', str(name))),               # 単純な数値形式も許容
                        f"測点名 '{name}' が正規化されていません"
                    )
    
    def test_single_file_without_sections(self):
        """区間なしCSVファイルのテスト"""
        # 区間なしファイルを読み込む
        result = self.processor.read_csv_file(self.real_csv_single)
        self.assertTrue(result)
        
        # 区間を取得（区間1として処理されるはず）
        sections = self.processor.get_available_sections()
        self.assertEqual(len(sections), 1)
        self.assertEqual(sections[0], '区間1')
        
        # 区間1として処理
        processed = self.processor.process_section_data('区間1')
        self.assertIsNone(processed)
        self.assertIsNone(self.processor.csv_data)
        


if __name__ == '__main__':
    unittest.main() 