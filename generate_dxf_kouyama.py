#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
Googleスプレッドシートからデータを取得してDXF面積展開図を生成するスクリプト
小山町1359-5付近の道路断面図（右幅員のみ）
"""
import sys
import io
from pathlib import Path
import pandas as pd
import requests
import ezdxf

# プロジェクトルートをパスに追加
sys.path.insert(0, str(Path(__file__).parent))

from src.dxf_draw_tenkaiz import draw_road_sections
from src.processing import fill_station_names


def download_sheet_as_csv(sheet_id: str, gid: str = None) -> pd.DataFrame:
    """
    GoogleスプレッドシートをCSVとしてダウンロード
    
    Args:
        sheet_id: スプレッドシートID
        gid: シートのgid（指定しない場合は最初のシート）
    
    Returns:
        DataFrame
    """
    if gid:
        url = f"https://docs.google.com/spreadsheets/d/{sheet_id}/export?format=csv&gid={gid}"
    else:
        url = f"https://docs.google.com/spreadsheets/d/{sheet_id}/export?format=csv"
    
    print(f"[INFO] スプレッドシートをダウンロード中: {url}")
    response = requests.get(url)
    response.raise_for_status()
    
    # CSVを読み込み（エンコーディングを指定）
    response.encoding = 'utf-8-sig'  # BOM付きUTF-8に対応
    df = pd.read_csv(io.StringIO(response.text))
    print(f"[INFO] データを取得しました: {len(df)}行")
    return df


def parse_area_calculation_sheet(df: pd.DataFrame) -> pd.DataFrame:
    """
    面積計算書のスプレッドシートを解析して、draw_road_sections用のDataFrameに変換
    右幅員のみを使用（左幅員は0）
    
    Returns:
        name, x, wl, wr カラムを持つDataFrame
    """
    # ヘッダー行を探す（「測点」を含む行）
    header_row = None
    for i, row in df.iterrows():
        row_values = [str(v) for v in row.values if pd.notna(v)]
        if any('測点' in str(v) for v in row.values if pd.notna(v)):
            header_row = i
            print(f"[INFO] ヘッダー行を検出: {header_row}行目")
            break
    
    if header_row is None:
        header_row = 0
        print("[WARN] ヘッダー行が見つかりません。最初の行を使用します。")
    
    # データ部分を抽出（ヘッダーの次の行から）
    data_df = df.iloc[header_row + 1:].copy()
    
    # 列名をマッピング
    header_cols = df.iloc[header_row].values
    data_df.columns = range(len(data_df.columns))
    
    # 列インデックスを検出
    name_col_idx = None
    distance_col_idx = None
    width_col_idx = None
    
    for idx, col_name in enumerate(header_cols):
        col_str = str(col_name).strip() if pd.notna(col_name) else ''
        if '測点' in col_str:
            name_col_idx = idx
            print(f"[INFO] 測点列を検出: {idx}列目 ({col_str})")
        elif '延長' in col_str or '距離' in col_str:
            distance_col_idx = idx
            print(f"[INFO] 距離列を検出: {idx}列目 ({col_str})")
        elif '幅員' in col_str and '平均' not in col_str:
            width_col_idx = idx
            print(f"[INFO] 幅員列を検出: {idx}列目 ({col_str})")
    
    # 列が見つからない場合は位置で推測
    if name_col_idx is None:
        name_col_idx = 0
        print("[WARN] 測点列が見つかりません。最初の列を使用します。")
    if distance_col_idx is None:
        distance_col_idx = 1
        print("[WARN] 距離列が見つかりません。2番目の列を使用します。")
    if width_col_idx is None:
        width_col_idx = 2
        print("[WARN] 幅員列が見つかりません。3番目の列を使用します。")
    
    # データを抽出
    data_df['name'] = data_df.iloc[:, name_col_idx].astype(str)
    data_df['distance'] = pd.to_numeric(data_df.iloc[:, distance_col_idx], errors='coerce')
    data_df['width'] = pd.to_numeric(data_df.iloc[:, width_col_idx], errors='coerce')
    
    # 測点名を正規化（NaNや空文字列を除外）
    data_df['name'] = data_df['name'].fillna('')
    
    # 「合計」行などを除外（距離または幅員がNaNの行）
    data_df = data_df[(data_df['distance'].notna()) | (data_df['width'].notna())]
    
    # 最初の行（0m地点）の距離がNaNの場合、0として扱う
    if len(data_df) > 0 and pd.isna(data_df.iloc[0]['distance']):
        data_df.iloc[0, data_df.columns.get_loc('distance')] = 0.0
    
    # 距離がNaNの行を除外（最初の行以外）
    data_df = data_df[data_df['distance'].notna()]
    
    # 幅員がNaNの行を除外
    data_df = data_df[data_df['width'].notna()]
    
    # 累積距離を計算
    data_df['x'] = data_df['distance'].cumsum()
    
    # 右幅員のみを使用（左幅員は0）
    data_df['wl'] = 0.0  # 左側は0
    data_df['wr'] = data_df['width']  # 全幅を右側に
    
    # 結果のDataFrameを作成
    result_df = pd.DataFrame({
        'name': data_df['name'],
        'x': data_df['x'],
        'wl': data_df['wl'],
        'wr': data_df['wr']
    })
    
    # 測点名を補完
    result_df = fill_station_names(result_df)
    
    print(f"[INFO] データを変換しました: {len(result_df)}行")
    print(f"[INFO] 距離範囲: {result_df['x'].min():.2f}m ～ {result_df['x'].max():.2f}m")
    print(f"[INFO] 幅員範囲: 左 {result_df['wl'].max():.2f}m, 右 {result_df['wr'].max():.2f}m")
    
    return result_df


def generate_dxf_from_sheet(
    sheet_id: str,
    gid: str = None,
    output_path: str = None,
    title_block_data: dict = None
):
    """
    スプレッドシートからDXFを生成
    
    Args:
        sheet_id: GoogleスプレッドシートID
        gid: シートのgid
        output_path: 出力DXFファイルパス
        title_block_data: タイトルブロック情報
    """
    # スプレッドシートをダウンロード
    df = download_sheet_as_csv(sheet_id, gid)
    
    # データを変換
    data_df = parse_area_calculation_sheet(df)
    
    if data_df.empty:
        print("[ERROR] 有効なデータがありません")
        return
    
    # 出力パスを決定
    if output_path is None:
        output_path = "面積展開図_小山町.dxf"
    
    output_path = Path(output_path)
    
    # DXFを生成
    print(f"[INFO] DXFを生成中: {output_path}")
    doc = ezdxf.new("R2010")
    doc.header["$INSUNITS"] = 6  # metres
    doc.header["$MEASUREMENT"] = 1  # metric
    
    # スプレッドシートの1行目から路線名を抽出
    route_name = '小山町'  # デフォルト値
    for row_idx in [0, 1]:
        if len(df) > row_idx:
            row = df.iloc[row_idx]
            for cell_value in row:
                if pd.notna(cell_value):
                    cell_str = str(cell_value).strip()
                    # 「付近」を含むセルを探す
                    if '付近' in cell_str:
                        # 住所から地名を抽出（例：「熊本市東区小山町1359-5付近」→「小山町」）
                        if '小山町' in cell_str:
                            route_name = '小山町'
                        print(f"[INFO] 路線名を抽出: {route_name} (元の値: {cell_str})")
                        break
            if route_name != '小山町' or '小山町' in str(row.values):
                break
    
    # タイトルブロックデータを設定
    if title_block_data is None:
        title_block_data = {
            '工事名': '東区市道（２工区）舗装補修工事（水防等含）（単価契約）',
            '図面名': '面積展開図',
            '路線名': route_name,
            '作成日': pd.Timestamp.now().strftime('%Y年%m月%d日'),
            '縮尺': '1/200(A3)',
            '図面番号': '1/1',
            '施工者': ''
        }
    else:
        # title_block_dataが既に渡されている場合でも、抽出した路線名で上書き
        title_block_data['路線名'] = route_name
    
    # 描画
    draw_road_sections(
        doc.modelspace(),
        data_df,
        scale=1000.0,
        text_height=700.0,
        title_block_data=title_block_data,
        frame_scale_ratio=200
    )
    
    # 保存
    doc.saveas(output_path)
    print(f"[INFO] DXFを保存しました: {output_path}")


if __name__ == "__main__":
    # スプレッドシートIDを抽出
    sheet_url = "https://docs.google.com/spreadsheets/d/1hY3SKROKY-iYheV1ZB0gWZs683e_k0dP?rtpof=true&usp=drive_fs"
    
    # URLからIDを抽出
    sheet_id = "1hY3SKROKY-iYheV1ZB0gWZs683e_k0dP"
    
    # 出力先フォルダ（Googleドライブの同期フォルダ）
    output_dir = Path(r"H:\マイドライブ\〇東区市道（2工区）舗装補修工事（水防等含）（単価契約）\20251028小山町1359-5\作成中")
    
    # 出力パス（出力先フォルダ内に保存）
    output_path = output_dir / "面積展開図_小山町.dxf"
    
    try:
        generate_dxf_from_sheet(sheet_id, None, output_path)
        print("\n[SUCCESS] DXF生成が完了しました")
    except Exception as e:
        print(f"\n[ERROR] DXF生成に失敗しました: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
