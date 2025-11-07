#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
PDFプレビューGUIモジュール

PDFファイルをGUIで表示し、ユーザーによる承認・拒否を取得する。
"""

from typing import Optional, Callable
import os
import tkinter as tk
from tkinter import ttk

try:
    import fitz  # PyMuPDF
    PYMUPDF_AVAILABLE = True
except ImportError:
    PYMUPDF_AVAILABLE = False

try:
    from PIL import Image, ImageTk
    PIL_AVAILABLE = True
except ImportError:
    PIL_AVAILABLE = False


class PDFPreviewError(Exception):
    """PDFプレビュー関連のエラー"""
    pass


class PDFPreviewGUI:
    """PDFプレビューをGUIで表示するクラス"""

    def __init__(
        self,
        pdf_path: str,
        title: Optional[str] = None,
        window_width: int = 800,
        window_height: int = 1000,
        preview_width: int = 700,
        zoom_factor: float = 2.0
    ):
        """
        初期化

        Args:
            pdf_path: プレビュー対象のPDFファイルパス
            title: ウィンドウタイトル（Noneの場合はファイル名を使用）
            window_width: ウィンドウ幅
            window_height: ウィンドウ高さ
            preview_width: プレビュー画像の幅
            zoom_factor: PDFレンダリング時のズーム倍率

        Raises:
            PDFPreviewError: 必要なライブラリがインストールされていない、またはPDFが開けない
        """
        if not PYMUPDF_AVAILABLE:
            raise PDFPreviewError("PyMuPDF (fitz) がインストールされていません。'pip install PyMuPDF' でインストールしてください。")

        if not PIL_AVAILABLE:
            raise PDFPreviewError("Pillow がインストールされていません。'pip install Pillow' でインストールしてください。")

        if not os.path.exists(pdf_path):
            raise PDFPreviewError(f"PDFファイルが見つかりません: {pdf_path}")

        self.pdf_path = pdf_path
        self.title = title or f"PDFプレビュー - {os.path.basename(pdf_path)}"
        self.window_width = window_width
        self.window_height = window_height
        self.preview_width = preview_width
        self.zoom_factor = zoom_factor

        self.approved = False
        self.root = None
        self.images = []  # PIL Imageオブジェクトを保持（GCされないように）

    def _create_gui(self):
        """GUIウィンドウを構築"""
        # PDFを開く
        try:
            doc = fitz.open(self.pdf_path)
        except Exception as e:
            raise PDFPreviewError(f"PDFを開けませんでした: {e}")

        # GUIウィンドウを作成
        self.root = tk.Tk()
        self.root.title(self.title)
        self.root.geometry(f"{self.window_width}x{self.window_height}")

        # スクロール可能なキャンバス
        canvas = tk.Canvas(self.root, bg='gray')
        scrollbar = ttk.Scrollbar(self.root, orient="vertical", command=canvas.yview)
        scrollable_frame = ttk.Frame(canvas)

        scrollable_frame.bind(
            "<Configure>",
            lambda e: canvas.configure(scrollregion=canvas.bbox("all"))
        )

        canvas.create_window((0, 0), window=scrollable_frame, anchor="nw")
        canvas.configure(yscrollcommand=scrollbar.set)

        # 各ページを画像として表示
        page_count = len(doc)
        for page_num in range(page_count):
            page = doc[page_num]

            # ページを画像に変換
            mat = fitz.Matrix(self.zoom_factor, self.zoom_factor)
            pix = page.get_pixmap(matrix=mat, alpha=False)

            # PIL Imageに変換
            img_data = pix.tobytes("ppm")
            img = Image.frombytes("RGB", [pix.width, pix.height], img_data)

            # 表示サイズを調整
            width_percent = (self.preview_width / float(img.size[0]))
            new_height = int((float(img.size[1]) * float(width_percent)))
            img = img.resize((self.preview_width, new_height), Image.LANCZOS)

            self.images.append(img)

            # tkinter用のPhotoImageに変換
            photo = ImageTk.PhotoImage(img)

            # ページラベルとイメージを追加
            page_label = ttk.Label(
                scrollable_frame,
                text=f"ページ {page_num + 1}/{page_count}",
                font=('Arial', 10, 'bold')
            )
            page_label.pack(pady=(10, 5))

            img_label = ttk.Label(scrollable_frame, image=photo)
            img_label.image = photo  # 参照を保持
            img_label.pack(pady=5)

        # ボタンフレーム
        button_frame = ttk.Frame(self.root)
        button_frame.pack(side=tk.BOTTOM, fill=tk.X, pady=10)

        def on_approve():
            self.approved = True
            self.root.quit()
            self.root.destroy()

        def on_cancel():
            self.approved = False
            self.root.quit()
            self.root.destroy()

        approve_btn = ttk.Button(
            button_frame,
            text="OK - 承認",
            command=on_approve
        )
        approve_btn.pack(side=tk.LEFT, padx=20)

        cancel_btn = ttk.Button(
            button_frame,
            text="キャンセル",
            command=on_cancel
        )
        cancel_btn.pack(side=tk.LEFT, padx=20)

        # レイアウト
        canvas.pack(side="left", fill="both", expand=True)
        scrollbar.pack(side="right", fill="y")

        doc.close()

    def show(self) -> bool:
        """
        プレビューウィンドウを表示し、ユーザーの選択を取得

        Returns:
            承認された場合True、キャンセルされた場合False
        """
        self._create_gui()
        self.root.mainloop()
        return self.approved

    @staticmethod
    def show_pdf_and_get_approval(
        pdf_path: str,
        title: Optional[str] = None,
        on_approve: Optional[Callable[[], None]] = None,
        on_cancel: Optional[Callable[[], None]] = None
    ) -> bool:
        """
        PDFプレビューを表示して承認を取得（便利メソッド）

        Args:
            pdf_path: プレビュー対象のPDFファイルパス
            title: ウィンドウタイトル
            on_approve: 承認時に呼び出されるコールバック
            on_cancel: キャンセル時に呼び出されるコールバック

        Returns:
            承認された場合True、キャンセルされた場合False
        """
        preview = PDFPreviewGUI(pdf_path, title)
        approved = preview.show()

        if approved and on_approve:
            on_approve()
        elif not approved and on_cancel:
            on_cancel()

        return approved


def show_pdf_preview(pdf_path: str, title: Optional[str] = None) -> bool:
    """
    PDFプレビューを表示（関数インターフェース）

    Args:
        pdf_path: プレビュー対象のPDFファイルパス
        title: ウィンドウタイトル

    Returns:
        承認された場合True、キャンセルされた場合False
    """
    return PDFPreviewGUI.show_pdf_and_get_approval(pdf_path, title)


if __name__ == '__main__':
    import sys

    if len(sys.argv) < 2:
        print("使用方法: python pdf_preview_gui.py <PDF_PATH>")
        sys.exit(1)

    pdf_path = sys.argv[1]

    try:
        approved = show_pdf_preview(pdf_path)
        if approved:
            print("承認されました")
        else:
            print("キャンセルされました")
    except PDFPreviewError as e:
        print(f"エラー: {e}")
        sys.exit(1)
