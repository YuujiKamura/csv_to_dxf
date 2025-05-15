import os
import sys
import ezdxf

# 相対インポートのためのパス設定
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..')))
from src.dxf_draw_tenkaiz import draw_road_sections

class DXFGenerator:
    """DXFファイルの生成を担当するクラス"""
    
    def __init__(self):
        """初期化処理"""
        self.reset()
    
    def reset(self):
        """状態をリセット"""
        self.output_file = None
        self.dxf_doc = None
    
    def create_new_document(self, dxf_version="R2010"):
        """
        新しいDXFドキュメントを作成する
        
        Args:
            dxf_version: DXFのバージョン（デフォルト: R2010）
            
        Returns:
            bool: 成功したかどうか
        """
        try:
            # 新しいDXFドキュメントを作成
            self.dxf_doc = ezdxf.new(dxfversion=dxf_version)
            
            # 単位をメートルに設定
            self.dxf_doc.header['$INSUNITS'] = 6  # 6 is the code for meters
            self.dxf_doc.header['$MEASUREMENT'] = 1  # 1 is the code for metric units
            
            return True
        except Exception as e:
            print(f"DXFドキュメント作成エラー: {e}")
            return False
    
    def generate_dxf_from_dataframe(self, df, output_file, section_number=1):
        """
        DataFrameからDXFファイルを生成する
        
        Args:
            df: 処理済みのデータフレーム
            output_file: 出力ファイルのパス
            section_number: 区間番号（デフォルト: 1）
            
        Returns:
            bool: 生成に成功したかどうか
        """
        try:
            # ドキュメントの作成
            if not self.create_new_document():
                return False
                
            # 出力ファイルを保存
            self.output_file = output_file
            
            # モデル空間を取得
            msp = self.dxf_doc.modelspace()
            
            # 道路断面図を描画
            draw_road_sections(msp, df)
            
            # DXFファイルを保存
            self.dxf_doc.saveas(output_file)
            
            print(f"DXFファイルが生成されました: {output_file}")
            return True
        except Exception as e:
            print(f"DXF生成エラー: {e}")
            return False
    
    def generate_dxf_from_sections(self, sections_data, output_dir):
        """
        複数の区間データからDXFファイルを生成する
        
        Args:
            sections_data: 区間名をキー、DataFrameを値とする辞書
            output_dir: 出力ディレクトリのパス
            
        Returns:
            list: 生成されたDXFファイルのパスのリスト
        """
        generated_files = []
        
        try:
            for section_name, df in sections_data.items():
                if df is None or df.empty:
                    print(f"警告: {section_name}のデータが空です。スキップします。")
                    continue
                
                # 区間番号を抽出（例: '区間1' -> 1）
                section_number = 1
                if section_name[-1].isdigit():
                    section_number = int(section_name[-1])
                
                # 出力ファイルのパスを生成
                output_file = os.path.join(output_dir, f"{section_name}.dxf")
                
                # DXFファイルを生成
                if self.generate_dxf_from_dataframe(df, output_file, section_number):
                    generated_files.append(output_file)
            
            return generated_files
        except Exception as e:
            print(f"複数区間のDXF生成エラー: {e}")
            return generated_files 