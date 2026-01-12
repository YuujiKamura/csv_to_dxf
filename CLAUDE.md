# Claude Code 設定

## 自動化ルール

### コミット・PR
- 機能実装やバグ修正が完了したら、自動的にコミットしてプッシュする
- 意味のある単位でコミットをまとめる（細かすぎず、大きすぎず）
- PRが必要な場合は自動的に作成する（ユーザーの指示を待たない）

### ビルド
- Rust/Wasm の変更後は自動的に `wasm-pack build` を実行
- ビルドエラーがあれば修正してから再ビルド

## プロジェクト構成

```
csv_to_dxf/
├── src/                    # Python - CSV→DXF変換
├── gui/                    # Python - PyQt6 GUI
├── web/                    # Rust/Wasm - Webビューア
│   ├── src/               # Rust ソースコード
│   └── www/               # 静的ファイル（GitHub Pages用）
└── data/                   # サンプルデータ
```

## 技術スタック

- **Python**: CSV処理、DXF生成（ezdxf）、GUI（PyQt6）
- **Rust + WebAssembly**: Webビューア、DXFパーサー
- **GitHub Actions**: 自動ビルド・デプロイ

## 関連リポジトリ

- trianglelist: Kotlin Multiplatform版（Android/Desktop）
  - DXFパーサーロジックの移植元
