#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Google Sheets PDFエクスポート汎用モジュール

Google スプレッドシートの特定シートをPDF化し、
オプションでプレビュー表示、Google Driveへのアップロードを行う。
"""

from typing import Optional, Tuple, List, Dict, Any, Callable
from dataclasses import dataclass
from enum import Enum
import io
import os
import tempfile
import requests
from urllib.parse import urlencode

from google.oauth2.credentials import Credentials
from google.auth.transport.requests import Request
from googleapiclient.discovery import build
from googleapiclient.http import MediaIoBaseUpload


class PageOrientation(Enum):
    """PDFページの向き"""
    PORTRAIT = "portrait"  # 縦
    LANDSCAPE = "landscape"  # 横


class PaperSize(Enum):
    """PDF用紙サイズ"""
    A4 = "A4"
    A3 = "A3"
    LETTER = "LETTER"
    LEGAL = "LEGAL"


@dataclass
class PDFExportConfig:
    """PDFエクスポート設定"""
    orientation: PageOrientation = PageOrientation.PORTRAIT
    paper_size: PaperSize = PaperSize.A4
    fit_to_width: bool = True
    fit_to_height: bool = True
    scale: int = 4
    show_gridlines: bool = False
    show_sheet_names: bool = False
    show_page_numbers: bool = False
    # マージン（インチ単位、1cm = 0.3937インチ）
    top_margin: float = 0.39
    bottom_margin: float = 0.39
    left_margin: float = 0.39
    right_margin: float = 0.39
    # セル内容の配置
    horizontal_alignment: str = "CENTER"
    vertical_alignment: str = "MIDDLE"


@dataclass
class SheetSelector:
    """シート選択の条件"""
    # 以下のいずれか一つを指定
    sheet_name: Optional[str] = None  # 完全一致
    sheet_name_prefix: Optional[str] = None  # 前方一致
    sheet_name_contains: Optional[str] = None  # 部分一致
    sheet_id: Optional[int] = None  # シートID直接指定
    # 複数マッチ時の選択方法
    select_latest: bool = False  # 名前に番号が含まれる場合、最大のものを選択
    sheet_filter: Optional[Callable[[Dict[str, Any]], bool]] = None  # カスタムフィルタ


class SheetsAPIError(Exception):
    """Sheets API関連のエラー"""
    pass


class SheetsPDFExporter:
    """Google SheetsからPDFへのエクスポート機能を提供するクラス"""

    def __init__(self, credentials: Credentials):
        """
        初期化

        Args:
            credentials: Google API認証情報
        """
        self.credentials = credentials
        self.sheets_service = build('sheets', 'v4', credentials=credentials)
        self.drive_service = None  # 必要時に初期化

    def _ensure_drive_service(self):
        """Drive APIサービスを初期化（遅延初期化）"""
        if self.drive_service is None:
            self.drive_service = build('drive', 'v3', credentials=self.credentials)

    def _column_index_to_letter(self, index: int) -> str:
        """
        列インデックス（1始まり）をA1表記の文字に変換

        Args:
            index: 1始まりの列インデックス

        Returns:
            A1表記の列文字（例: 1→'A', 27→'AA'）
        """
        if index < 1:
            return 'A'
        result = ''
        while index:
            index, remainder = divmod(index - 1, 26)
            result = chr(65 + remainder) + result
        return result

    def _cell_has_border(self, cell: Dict[str, Any]) -> bool:
        """
        セルに明示的な罫線が設定されているか判定

        Args:
            cell: Sheets APIから取得したセル情報

        Returns:
            罫線があればTrue
        """
        if not cell:
            return False

        for key in ('effectiveFormat', 'userEnteredFormat'):
            fmt = cell.get(key) or {}
            borders = fmt.get('borders') or {}
            for edge in ('top', 'bottom', 'left', 'right', 'innerHorizontal', 'innerVertical'):
                edge_fmt = borders.get(edge) or {}
                style = edge_fmt.get('style')
                if style and style != 'NONE':
                    return True
        return False

    def _compute_used_range(
        self,
        values: List[List[Any]],
        grid_row_data: List[Dict[str, Any]]
    ) -> Tuple[int, int]:
        """
        データと罫線情報から実際に使用されている範囲を計算

        Args:
            values: セルの値（2次元配列）
            grid_row_data: Sheets APIから取得したグリッドデータ

        Returns:
            (最終行インデックス, 最終列インデックス) ※0始まり
        """
        last_row = 0
        last_col = 0
        has_usage = False
        max_rows = max(len(values), len(grid_row_data))

        for row_index in range(max_rows):
            row_values = values[row_index] if row_index < len(values) else []
            row_formats_entry = grid_row_data[row_index] if row_index < len(grid_row_data) else {}
            row_formats = row_formats_entry.get('values', []) if row_formats_entry else []
            row_used = False
            max_cols = max(len(row_values), len(row_formats))

            for col_index in range(max_cols):
                cell_value = row_values[col_index] if col_index < len(row_values) else None
                if isinstance(cell_value, str):
                    cell_str = cell_value.strip()
                elif cell_value is None:
                    cell_str = ''
                else:
                    cell_str = str(cell_value).strip()
                has_value = bool(cell_str)

                cell_format = row_formats[col_index] if col_index < len(row_formats) else {}
                has_border = self._cell_has_border(cell_format)

                if has_value or has_border:
                    row_used = True
                    has_usage = True
                    if col_index > last_col:
                        last_col = col_index

            if row_used:
                last_row = row_index

        if not has_usage:
            return 0, 0

        return last_row, last_col

    def get_spreadsheet_info(self, spreadsheet_id: str) -> Dict[str, Any]:
        """
        スプレッドシートの基本情報を取得

        Args:
            spreadsheet_id: スプレッドシートID

        Returns:
            スプレッドシート情報
        """
        try:
            return self.sheets_service.spreadsheets().get(
                spreadsheetId=spreadsheet_id
            ).execute()
        except Exception as e:
            raise SheetsAPIError(f"スプレッドシート情報の取得に失敗: {e}")

    def find_sheets(
        self,
        spreadsheet_id: str,
        selector: SheetSelector
    ) -> List[Dict[str, Any]]:
        """
        条件に合致するシートを検索

        Args:
            spreadsheet_id: スプレッドシートID
            selector: シート選択条件

        Returns:
            マッチしたシートのプロパティのリスト
        """
        spreadsheet = self.get_spreadsheet_info(spreadsheet_id)
        sheets = spreadsheet.get('sheets', [])
        matched_sheets = []

        for sheet in sheets:
            properties = sheet.get('properties', {})
            title = properties.get('title', '')
            sheet_id = properties.get('sheetId', 0)

            # 条件チェック
            if selector.sheet_id is not None:
                if sheet_id == selector.sheet_id:
                    matched_sheets.append(properties)
            elif selector.sheet_name is not None:
                if title == selector.sheet_name:
                    matched_sheets.append(properties)
            elif selector.sheet_name_prefix is not None:
                if title.startswith(selector.sheet_name_prefix):
                    matched_sheets.append(properties)
            elif selector.sheet_name_contains is not None:
                if selector.sheet_name_contains in title:
                    matched_sheets.append(properties)

            # カスタムフィルタ
            if selector.sheet_filter and selector.sheet_filter(properties):
                if properties not in matched_sheets:
                    matched_sheets.append(properties)

        # 最新のものを選択（番号が含まれる場合）
        if selector.select_latest and len(matched_sheets) > 1:
            def extract_number(props):
                title = props.get('title', '')
                # タイトルから数値を抽出
                import re
                numbers = re.findall(r'\d+', title)
                return int(numbers[-1]) if numbers else 0

            matched_sheets.sort(key=extract_number, reverse=True)

        return matched_sheets

    def export_sheet_to_pdf(
        self,
        spreadsheet_id: str,
        sheet_selector: SheetSelector,
        config: Optional[PDFExportConfig] = None,
        auto_detect_range: bool = True
    ) -> bytes:
        """
        指定されたシートをPDFとしてエクスポート

        Args:
            spreadsheet_id: スプレッドシートID
            sheet_selector: シート選択条件
            config: PDFエクスポート設定（Noneの場合はデフォルト）
            auto_detect_range: データ範囲を自動検出するか

        Returns:
            PDFのバイトデータ

        Raises:
            SheetsAPIError: シートが見つからない、またはエクスポートに失敗
        """
        if config is None:
            config = PDFExportConfig()

        # シートを検索
        matched_sheets = self.find_sheets(spreadsheet_id, sheet_selector)

        if not matched_sheets:
            raise SheetsAPIError("条件に合致するシートが見つかりませんでした")

        target_sheet = matched_sheets[0]
        sheet_id = target_sheet.get('sheetId', 0)
        sheet_title = target_sheet.get('title', '')

        # アクセストークンを取得
        if not self.credentials.valid:
            self.credentials.refresh(Request())
        access_token = self.credentials.token

        # エクスポートパラメータを構築
        export_params = {
            'format': 'pdf',
            'gid': sheet_id,
            'portrait': 'true' if config.orientation == PageOrientation.PORTRAIT else 'false',
            'size': config.paper_size.value,
            'fitw': 'true' if config.fit_to_width else 'false',
            'fith': 'true' if config.fit_to_height else 'false',
            'scale': str(config.scale),
            'sheetnames': 'true' if config.show_sheet_names else 'false',
            'printtitle': 'false',
            'pagenum': 'UNDEFINED' if not config.show_page_numbers else 'CENTER',
            'gridlines': 'true' if config.show_gridlines else 'false',
            'fzr': 'true',
            'attachment': 'false',
            'top_margin': config.top_margin,
            'bottom_margin': config.bottom_margin,
            'left_margin': config.left_margin,
            'right_margin': config.right_margin,
            'horizontal_alignment': config.horizontal_alignment,
            'vertical_alignment': config.vertical_alignment,
        }

        # データ範囲を自動検出
        if auto_detect_range:
            grid_properties = target_sheet.get('gridProperties', {})
            row_count = grid_properties.get('rowCount') or 1
            column_count = grid_properties.get('columnCount') or 1
            sheet_title_escaped = sheet_title.replace("'", "''")
            end_column_letter = self._column_index_to_letter(column_count)
            fetch_range = f"'{sheet_title_escaped}'!A1:{end_column_letter}{row_count}"

            # 値を取得
            result = self.sheets_service.spreadsheets().values().get(
                spreadsheetId=spreadsheet_id,
                range=fetch_range,
                valueRenderOption='FORMATTED_VALUE'
            ).execute()
            values = result.get('values', [])

            # 罫線情報を取得
            grid_response = self.sheets_service.spreadsheets().get(
                spreadsheetId=spreadsheet_id,
                ranges=[fetch_range],
                includeGridData=True,
                fields='sheets(properties.sheetId,data.rowData.values(effectiveFormat.borders,userEnteredFormat.borders))'
            ).execute()

            grid_row_data = []
            for sheet_data in grid_response.get('sheets', []):
                properties = sheet_data.get('properties', {})
                if properties.get('sheetId') == sheet_id:
                    data_blocks = sheet_data.get('data', [])
                    if data_blocks:
                        grid_row_data = data_blocks[0].get('rowData', [])
                    break

            # 使用範囲を計算
            last_row_idx, last_col_idx = self._compute_used_range(values, grid_row_data)
            export_params['r1'] = 0
            export_params['r2'] = last_row_idx + 1
            export_params['c1'] = 0
            export_params['c2'] = last_col_idx + 1

        # PDFをエクスポート
        export_url = f'https://docs.google.com/spreadsheets/d/{spreadsheet_id}/export?{urlencode(export_params)}'
        headers = {'Authorization': f'Bearer {access_token}'}
        response = requests.get(export_url, headers=headers)

        if response.status_code != 200:
            raise SheetsAPIError(f'PDFエクスポートに失敗: HTTP {response.status_code}')

        return response.content

    def export_and_save(
        self,
        spreadsheet_id: str,
        sheet_selector: SheetSelector,
        output_path: str,
        config: Optional[PDFExportConfig] = None,
        auto_detect_range: bool = True
    ) -> str:
        """
        シートをPDF化してファイルに保存

        Args:
            spreadsheet_id: スプレッドシートID
            sheet_selector: シート選択条件
            output_path: 出力ファイルパス
            config: PDFエクスポート設定
            auto_detect_range: データ範囲を自動検出するか

        Returns:
            保存されたファイルパス
        """
        pdf_bytes = self.export_sheet_to_pdf(
            spreadsheet_id,
            sheet_selector,
            config,
            auto_detect_range
        )

        with open(output_path, 'wb') as f:
            f.write(pdf_bytes)

        return output_path

    def export_to_temp_file(
        self,
        spreadsheet_id: str,
        sheet_selector: SheetSelector,
        config: Optional[PDFExportConfig] = None,
        auto_detect_range: bool = True,
        prefix: str = 'sheets_pdf_',
        suffix: str = '.pdf'
    ) -> str:
        """
        シートをPDF化して一時ファイルに保存

        Args:
            spreadsheet_id: スプレッドシートID
            sheet_selector: シート選択条件
            config: PDFエクスポート設定
            auto_detect_range: データ範囲を自動検出するか
            prefix: 一時ファイル名のプレフィックス
            suffix: 一時ファイル名のサフィックス

        Returns:
            一時ファイルのパス（呼び出し側で削除する責任あり）
        """
        pdf_bytes = self.export_sheet_to_pdf(
            spreadsheet_id,
            sheet_selector,
            config,
            auto_detect_range
        )

        fd, temp_path = tempfile.mkstemp(suffix=suffix, prefix=prefix)
        os.close(fd)

        with open(temp_path, 'wb') as f:
            f.write(pdf_bytes)

        return temp_path

    def upload_to_drive(
        self,
        pdf_data: bytes,
        filename: str,
        folder_id: Optional[str] = None,
        replace_existing: bool = False
    ) -> str:
        """
        PDFをGoogle Driveにアップロード

        Args:
            pdf_data: PDFのバイトデータ
            filename: アップロード時のファイル名
            folder_id: アップロード先フォルダID（Noneの場合はマイドライブのルート）
            replace_existing: 同名ファイルが存在する場合に置き換えるか

        Returns:
            アップロードされたファイルのID

        Raises:
            SheetsAPIError: アップロードに失敗
        """
        self._ensure_drive_service()

        # 同名ファイルの確認
        existing_file_id = None
        if replace_existing and folder_id:
            query = f"name='{filename}' and '{folder_id}' in parents and trashed=false"
            results = self.drive_service.files().list(
                q=query,
                fields='files(id, name)',
                pageSize=1
            ).execute()
            files = results.get('files', [])
            if files:
                existing_file_id = files[0]['id']

        file_metadata = {'name': filename}
        if folder_id:
            file_metadata['parents'] = [folder_id]

        media = MediaIoBaseUpload(
            io.BytesIO(pdf_data),
            mimetype='application/pdf',
            resumable=True
        )

        try:
            if existing_file_id:
                # 既存ファイルを更新
                updated_file = self.drive_service.files().update(
                    fileId=existing_file_id,
                    media_body=media,
                    fields='id, name, webViewLink'
                ).execute()
                return updated_file['id']
            else:
                # 新規ファイル作成
                uploaded_file = self.drive_service.files().create(
                    body=file_metadata,
                    media_body=media,
                    fields='id, name, webViewLink'
                ).execute()
                return uploaded_file['id']
        except Exception as e:
            raise SheetsAPIError(f'Driveへのアップロードに失敗: {e}')

    def export_and_upload(
        self,
        spreadsheet_id: str,
        sheet_selector: SheetSelector,
        filename: str,
        folder_id: Optional[str] = None,
        config: Optional[PDFExportConfig] = None,
        auto_detect_range: bool = True,
        replace_existing: bool = False
    ) -> str:
        """
        シートをPDF化してGoogle Driveにアップロード（ワンステップ）

        Args:
            spreadsheet_id: スプレッドシートID
            sheet_selector: シート選択条件
            filename: アップロード時のファイル名
            folder_id: アップロード先フォルダID
            config: PDFエクスポート設定
            auto_detect_range: データ範囲を自動検出するか
            replace_existing: 同名ファイルが存在する場合に置き換えるか

        Returns:
            アップロードされたファイルのID
        """
        pdf_bytes = self.export_sheet_to_pdf(
            spreadsheet_id,
            sheet_selector,
            config,
            auto_detect_range
        )

        return self.upload_to_drive(
            pdf_bytes,
            filename,
            folder_id,
            replace_existing
        )
