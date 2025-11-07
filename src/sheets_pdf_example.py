#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Google Sheets PDF エクスポートモジュールの使用例
"""

import os
import sys
from google.oauth2.credentials import Credentials

# モジュールをインポート
from sheets_pdf_exporter import (
    SheetsPDFExporter,
    SheetSelector,
    PDFExportConfig,
    PageOrientation,
    PaperSize,
    SheetsAPIError
)
from pdf_preview_gui import show_pdf_preview, PDFPreviewError


def example1_simple_export():
    """例1: シンプルなPDFエクスポート（シート名指定）"""
    print("=" * 80)
    print("例1: シンプルなPDFエクスポート")
    print("=" * 80)

    # 認証情報を読み込み
    TOKEN_FILE = r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\gmail_token.json"
    SCOPES = [
        'https://www.googleapis.com/auth/drive',
        'https://www.googleapis.com/auth/spreadsheets'
    ]

    creds = Credentials.from_authorized_user_file(TOKEN_FILE, SCOPES)

    # エクスポーターを作成
    exporter = SheetsPDFExporter(creds)

    # スプレッドシートIDとシート名を指定
    spreadsheet_id = '1RqW57aq7-QnoH37rg0TxivyiejJ6uwYBnW8iQGGh7Ac'
    sheet_selector = SheetSelector(sheet_name='Sheet1')  # シート名を直接指定

    # PDFとして保存
    output_path = 'output_example1.pdf'
    try:
        exporter.export_and_save(spreadsheet_id, sheet_selector, output_path)
        print(f"✓ PDFを保存しました: {output_path}")
    except SheetsAPIError as e:
        print(f"✗ エラー: {e}")


def example2_prefix_match_and_latest():
    """例2: プレフィックスマッチで最新シートを選択"""
    print("\n" + "=" * 80)
    print("例2: プレフィックスマッチで最新シートを選択")
    print("=" * 80)

    TOKEN_FILE = r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\gmail_token.json"
    SCOPES = [
        'https://www.googleapis.com/auth/drive',
        'https://www.googleapis.com/auth/spreadsheets'
    ]

    creds = Credentials.from_authorized_user_file(TOKEN_FILE, SCOPES)
    exporter = SheetsPDFExporter(creds)

    spreadsheet_id = '1RqW57aq7-QnoH37rg0TxivyiejJ6uwYBnW8iQGGh7Ac'

    # 'shuuho_' で始まるシートのうち、番号が最大のものを選択
    sheet_selector = SheetSelector(
        sheet_name_prefix='shuuho_',
        select_latest=True  # 最新（番号が最大）のものを選択
    )

    output_path = 'output_example2.pdf'
    try:
        exporter.export_and_save(spreadsheet_id, sheet_selector, output_path)
        print(f"✓ PDFを保存しました: {output_path}")
    except SheetsAPIError as e:
        print(f"✗ エラー: {e}")


def example3_custom_config():
    """例3: カスタム設定でエクスポート"""
    print("\n" + "=" * 80)
    print("例3: カスタム設定でエクスポート（横向き、A3）")
    print("=" * 80)

    TOKEN_FILE = r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\gmail_token.json"
    SCOPES = [
        'https://www.googleapis.com/auth/drive',
        'https://www.googleapis.com/auth/spreadsheets'
    ]

    creds = Credentials.from_authorized_user_file(TOKEN_FILE, SCOPES)
    exporter = SheetsPDFExporter(creds)

    spreadsheet_id = '1RqW57aq7-QnoH37rg0TxivyiejJ6uwYBnW8iQGGh7Ac'
    sheet_selector = SheetSelector(sheet_name='Sheet1')

    # カスタムPDF設定
    config = PDFExportConfig(
        orientation=PageOrientation.LANDSCAPE,  # 横向き
        paper_size=PaperSize.A3,  # A3サイズ
        show_gridlines=True,  # グリッド線を表示
        top_margin=0.5,  # 上マージン（インチ）
        bottom_margin=0.5,
        left_margin=0.5,
        right_margin=0.5,
    )

    output_path = 'output_example3.pdf'
    try:
        exporter.export_and_save(
            spreadsheet_id,
            sheet_selector,
            output_path,
            config=config
        )
        print(f"✓ PDFを保存しました: {output_path}")
    except SheetsAPIError as e:
        print(f"✗ エラー: {e}")


def example4_with_preview():
    """例4: プレビュー表示してからアップロード"""
    print("\n" + "=" * 80)
    print("例4: プレビュー表示してからGoogle Driveにアップロード")
    print("=" * 80)

    TOKEN_FILE = r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\gmail_token.json"
    SCOPES = [
        'https://www.googleapis.com/auth/drive',
        'https://www.googleapis.com/auth/spreadsheets'
    ]

    creds = Credentials.from_authorized_user_file(TOKEN_FILE, SCOPES)
    exporter = SheetsPDFExporter(creds)

    spreadsheet_id = '1RqW57aq7-QnoH37rg0TxivyiejJ6uwYBnW8iQGGh7Ac'
    sheet_selector = SheetSelector(
        sheet_name_prefix='shuuho_',
        select_latest=True
    )

    try:
        # 一時ファイルにエクスポート
        print("PDFを生成中...")
        temp_pdf = exporter.export_to_temp_file(spreadsheet_id, sheet_selector)
        print(f"✓ 一時ファイルに保存: {temp_pdf}")

        # プレビューを表示
        print("\nGUIプレビューを表示します...")
        approved = show_pdf_preview(temp_pdf, title="工事週報プレビュー")

        if approved:
            print("\n承認されました。Google Driveにアップロード中...")

            # PDFデータを読み込み
            with open(temp_pdf, 'rb') as f:
                pdf_data = f.read()

            # Driveにアップロード
            DRIVE_FOLDER_ID = '18myb0cJ71imEuGyZnz6eAA8a7YLMri--'
            file_id = exporter.upload_to_drive(
                pdf_data,
                '工事週報.pdf',
                folder_id=DRIVE_FOLDER_ID,
                replace_existing=True  # 同名ファイルを置き換え
            )
            print(f"✓ アップロード完了。ファイルID: {file_id}")
        else:
            print("\nキャンセルされました。アップロードは行いません。")

        # 一時ファイルを削除
        os.remove(temp_pdf)

    except (SheetsAPIError, PDFPreviewError) as e:
        print(f"✗ エラー: {e}")


def example5_direct_upload():
    """例5: プレビューなしで直接アップロード"""
    print("\n" + "=" * 80)
    print("例5: プレビューなしで直接Google Driveにアップロード")
    print("=" * 80)

    TOKEN_FILE = r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\gmail_token.json"
    SCOPES = [
        'https://www.googleapis.com/auth/drive',
        'https://www.googleapis.com/auth/spreadsheets'
    ]

    creds = Credentials.from_authorized_user_file(TOKEN_FILE, SCOPES)
    exporter = SheetsPDFExporter(creds)

    spreadsheet_id = '1RqW57aq7-QnoH37rg0TxivyiejJ6uwYBnW8iQGGh7Ac'
    sheet_selector = SheetSelector(
        sheet_name_prefix='shuuho_',
        select_latest=True
    )

    DRIVE_FOLDER_ID = '18myb0cJ71imEuGyZnz6eAA8a7YLMri--'

    try:
        print("PDFを生成してアップロード中...")
        file_id = exporter.export_and_upload(
            spreadsheet_id,
            sheet_selector,
            '工事週報_直接アップロード.pdf',
            folder_id=DRIVE_FOLDER_ID,
            replace_existing=True
        )
        print(f"✓ アップロード完了。ファイルID: {file_id}")
    except SheetsAPIError as e:
        print(f"✗ エラー: {e}")


def example6_sheet_id_direct():
    """例6: シートIDを直接指定"""
    print("\n" + "=" * 80)
    print("例6: シートIDを直接指定してエクスポート")
    print("=" * 80)

    TOKEN_FILE = r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\gmail_token.json"
    SCOPES = [
        'https://www.googleapis.com/auth/drive',
        'https://www.googleapis.com/auth/spreadsheets'
    ]

    creds = Credentials.from_authorized_user_file(TOKEN_FILE, SCOPES)
    exporter = SheetsPDFExporter(creds)

    spreadsheet_id = '1RqW57aq7-QnoH37rg0TxivyiejJ6uwYBnW8iQGGh7Ac'

    # シートIDを直接指定（ブラウザのURL末尾 #gid=123456789 の部分）
    sheet_selector = SheetSelector(sheet_id=0)  # 0は通常、最初のシート

    output_path = 'output_example6.pdf'
    try:
        exporter.export_and_save(spreadsheet_id, sheet_selector, output_path)
        print(f"✓ PDFを保存しました: {output_path}")
    except SheetsAPIError as e:
        print(f"✗ エラー: {e}")


def example7_custom_filter():
    """例7: カスタムフィルタでシートを選択"""
    print("\n" + "=" * 80)
    print("例7: カスタムフィルタでシートを選択")
    print("=" * 80)

    TOKEN_FILE = r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\gmail_token.json"
    SCOPES = [
        'https://www.googleapis.com/auth/drive',
        'https://www.googleapis.com/auth/spreadsheets'
    ]

    creds = Credentials.from_authorized_user_file(TOKEN_FILE, SCOPES)
    exporter = SheetsPDFExporter(creds)

    spreadsheet_id = '1RqW57aq7-QnoH37rg0TxivyiejJ6uwYBnW8iQGGh7Ac'

    # カスタムフィルタ：非表示でないシートのみ
    def not_hidden_filter(sheet_props):
        return not sheet_props.get('hidden', False)

    sheet_selector = SheetSelector(
        sheet_name_prefix='shuuho_',
        sheet_filter=not_hidden_filter
    )

    output_path = 'output_example7.pdf'
    try:
        exporter.export_and_save(spreadsheet_id, sheet_selector, output_path)
        print(f"✓ PDFを保存しました: {output_path}")
    except SheetsAPIError as e:
        print(f"✗ エラー: {e}")


if __name__ == '__main__':
    print("Google Sheets PDF エクスポートモジュール - 使用例\n")

    # 実行したい例を選択
    examples = {
        '1': ('シンプルなPDFエクスポート', example1_simple_export),
        '2': ('プレフィックスマッチで最新シート', example2_prefix_match_and_latest),
        '3': ('カスタム設定（横向き、A3）', example3_custom_config),
        '4': ('プレビュー表示してアップロード', example4_with_preview),
        '5': ('直接アップロード', example5_direct_upload),
        '6': ('シートID直接指定', example6_sheet_id_direct),
        '7': ('カスタムフィルタ', example7_custom_filter),
        'all': ('すべて実行', None),
    }

    if len(sys.argv) > 1:
        choice = sys.argv[1]
    else:
        print("使用例を選択してください:")
        for key, (desc, _) in examples.items():
            print(f"  {key}: {desc}")
        print()
        choice = input("選択 (1-7 または all): ").strip()

    if choice == 'all':
        for key, (desc, func) in examples.items():
            if key != 'all' and func:
                func()
    elif choice in examples and examples[choice][1]:
        examples[choice][1]()
    else:
        print("無効な選択です")
        sys.exit(1)

    print("\n完了")
