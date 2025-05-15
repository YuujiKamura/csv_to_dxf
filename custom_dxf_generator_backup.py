#!/usr/bin/env python
# -*- coding: utf-8 -*-

import os
import sys
import argparse
from pathlib import Path

# 相対インポートのためのパス設定
csv_to_dxf_path = Path(__file__).parent / "csv_to_dxf-master"
sys.path.append(str(csv_to_dxf_path.absolute()))

from gui.core.csv_processor import CSVProcessor
from src.processing import get_available_sections
import src.dxf_draw_tenkaiz as draw
from src.exporter import export_dxf

def main():
    """CSVからDXFを生成するメインスクリプト"""
    parser = argparse.ArgumentParser(description='CSVファイルからDXFを生成します')
    parser.add_argument('--input', '-i', default='おやま.csv', help='入力CSVファイルのパス')
    parser.add_argument('--output', '-o', default='おやま_右幅員.dxf', help='出力DXFファイル名')
    parser.add_argument('--text-size', '-t', type=float, default=350.0, help='テキストサイズ (デフォルト: 350.0)')
    parser.add_argument('--scale', '-s', type=float, default=1000.0, help='スケール倍率 (デフォルト: 1000.0)')
    parser.add_argument('--assign', '-a', choices=['wl', 'wr', 'none'], default='wr', 
                        help='幅員の割り当て方法 (wl: 左側, wr: 右側, none: 割り当てなし)')
    
    args = parser.parse_args()
    
    # 入力CSVファイルのパス
    input_file = args.input
    # 出力DXFファイルの保存先
    output_file = args.output
    # テキストサイズ
    text_height = args.text_size
    # スケール
    scale = args.scale
    # 幅員割り当て
    width_assign = None if args.assign == 'none' else args.assign
    
    # 現在のディレクトリを確認
    current_dir = os.getcwd()
    print(f"現在のディレクトリ: {current_dir}")
    
    # 入力ファイルの絶対パス
    input_path = os.path.join(current_dir, input_file)
    print(f"入力CSVファイル: {input_path}")
    print(f"出力DXFファイル: {output_file}")
    print(f"テキストサイズ: {text_height}")
    print(f"スケール倍率: {scale}")
    print(f"幅員割り当て: {args.assign}")
    
    # ファイルの存在確認
    if not os.path.exists(input_path):
        print(f"エラー: 入力ファイル「{input_path}」が存在しません")
        return False
    
    # CSVプロセッサーを初期化
    processor = CSVProcessor()
    
    # ファイルを読み込む
    if not processor.read_csv_file(input_path):
        print(f"CSVファイル「{input_path}」の読み込みに失敗しました")
        return False
    
    # 区間名を取得（デフォルトは区間1）
    section_name = "区間1"
    sections = processor.get_available_sections()
    if sections:
        print(f"利用可能な区間: {sections}")
        section_name = sections[0]
    
    # 区間データを処理
    df = processor.process_section_data(
        section_name, 
        auto_convert=True,
        normalize_station_names=True,
        width_assign=width_assign
    )
    
    if df is None or df.empty:
        print(f"区間「{section_name}」のデータ処理に失敗しました")
        return False
    
    print(f"処理されたデータ: {len(df)}行")
    if width_assign:
        side = "左側" if width_assign == "wl" else "右側"
        print(f"幅員を{side}に割り当てました")
    
    # 出力ファイルのパスを作成
    output_path = os.path.join(current_dir, output_file)
    
    try:
        # ここでテキストサイズとスケールを指定して描画関数を作成
        def custom_draw_func(msp, data):
            return draw.draw_road_sections(msp, data, scale=scale, text_height=text_height)
        
        # DXFファイルを直接生成
        print(f"[INFO] DXF出力: {output_path}")
        export_dxf(df, output_path, custom_draw_func)
        print(f"[INFO] DXF保存成功: {output_path}")
        print(f"テキストサイズ {text_height} で図面を生成しました")
        return True
    except Exception as e:
        print(f"[ERROR] DXF 保存に失敗: {e}")
        return False

if __name__ == "__main__":
    main() 