import os
import sys
import unittest
from pathlib import Path
import shutil
import tempfile
import logging

# プロジェクトルートをパスに追加
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

# アプリケーションのモジュールをインポート
from gui.app_controller import AppController
import ezdxf


class TestSaveDXFFlow(unittest.TestCase):
    """DXF保存機能のフロー全体をテストする"""
    
    def setUp(self):
        """テスト前の準備"""
        # 一時出力ディレクトリを作成
        self.temp_dir = Path(os.getcwd()) / "test_output"
        self.temp_dir.mkdir(exist_ok=True)
        
        # AppControllerのインスタンスを作成
        self.controller = AppController()
        
        # 出力ディレクトリを設定する
        self.controller.output_dir = self.temp_dir
        
        # テスト用CSVファイルのパス
        self.data_dir = Path(__file__).parents[1] / 'data'
        self.test_files = {
            '区間1': self.data_dir / '区間1.csv',
            '区間3': self.data_dir / '区間3.csv',
            '区間5': self.data_dir / '区間5.csv',
            '区間6': self.data_dir / '区間6.csv',
        }
        
        # ロギングを設定（デバッグ時のみ必要）
        for handler in logging.root.handlers[:]:
            logging.root.removeHandler(handler)
        logging.basicConfig(
            level=logging.DEBUG,
            format="%(asctime)s [%(levelname)s] %(message)s",
            stream=sys.stdout
        )
        
    def tearDown(self):
        """テスト後のクリーンアップ"""
        # 一時ディレクトリを削除（必要な場合）
        # shutil.rmtree(self.temp_dir)
        pass
    
    def test_save_dxf_all_sections(self):
        """すべての区間データをDXFとして保存できることを確認"""
        
        print(f"\n===== DXF保存テスト開始 =====")
        print(f"テストCSVファイル: {self.test_files}")
        print(f"出力先ディレクトリ: {self.temp_dir}")
        
        results = []
        
        # 各区間ファイルについてテスト
        for section_name, csv_path in self.test_files.items():
            if not csv_path.exists():
                print(f"警告: ファイル {csv_path} が見つかりません。スキップします。")
                continue
                
            print(f"\n----- {section_name} の処理開始 -----")
            
            # ファイルロード
            load_result = self.controller.load_file(csv_path)
            self.assertTrue(load_result, f"ファイル {csv_path} の読み込みに失敗しました")
            print(f"CSVファイル読み込み成功: {csv_path}")
            
            # 区間の処理
            df = self.controller.process_section(section_name)
            self.assertIsNotNone(df, f"区間 {section_name} の処理に失敗しました")
            self.assertFalse(df.empty, f"区間 {section_name} のデータが空です")
            print(f"区間データ処理成功: {len(df)} 行")
            
            # DXF保存
            output_file = self.temp_dir / f"{section_name}_test.dxf"
            save_result = self.controller.save_dxf(output_file.name)
            
            # テスト結果
            self.assertTrue(save_result, f"DXF保存に失敗しました: {output_file}")
            self.assertTrue(output_file.exists(), f"DXFファイルが生成されませんでした: {output_file}")
            self.assertGreater(output_file.stat().st_size, 0, f"生成されたDXFファイルが空です: {output_file}")
            
            # DXFファイルが有効かをチェック
            try:
                dxf = ezdxf.readfile(output_file)
                msp = dxf.modelspace()
                entity_count = len(list(msp))
                print(f"DXF保存成功: {output_file} (サイズ: {output_file.stat().st_size} バイト, エンティティ数: {entity_count})")
                results.append((section_name, True, entity_count))
            except Exception as e:
                print(f"DXFファイル検証エラー: {e}")
                results.append((section_name, False, 0))
                self.fail(f"生成されたDXFファイルが無効です: {output_file}")
        
        # 結果のサマリー表示
        print("\n===== テスト結果サマリー =====")
        for section_name, success, entity_count in results:
            status = "成功" if success else "失敗"
            print(f"{section_name}: {status} (エンティティ数: {entity_count})")


if __name__ == '__main__':
    unittest.main() 