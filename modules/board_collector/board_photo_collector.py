#!/usr/bin/env python3
"""
ボード写真収集モジュール（汎用版）

YOLOモデルを使用してフォルダから出来型ボード写真を自動収集する。
ボード面積比率による分類、ハッシュによる重複除去をサポート。
"""

import sys
import json
import hashlib
from pathlib import Path
from typing import List, Dict, Tuple, Optional
from PIL import Image


class BoardPhotoCollector:
    """ボード写真収集クラス"""

    def __init__(self, yolo_predictor, board_classifier):
        """
        Args:
            yolo_predictor: YOLOPredictorインスタンス
            board_classifier: judge_caption_board_closeup関数
        """
        self.predictor = yolo_predictor
        self.classifier = board_classifier

    def get_image_files(self, folder: Path) -> List[Path]:
        """フォルダ内の画像ファイルを取得（再帰的、重複除去済み）"""
        extensions = ['.jpg', '.jpeg', '.JPG', '.JPEG']
        images = []

        for ext in extensions:
            images.extend(folder.rglob(f'*{ext}'))

        # 重複を除去（大文字小文字を区別しないファイルシステム対策）
        unique_images = list(dict.fromkeys(images))

        return sorted(unique_images)

    def detect_board_in_image(
        self, image_path: Path
    ) -> Tuple[Optional[bool], float]:
        """
        画像のボード検出と面積比率判定

        Returns:
            (is_closeup, ratio): 接写判定とボード面積比率
        """
        try:
            # 画像サイズを取得
            with Image.open(image_path) as img:
                img_w, img_h = img.size

            # YOLO検出
            bboxes = self.predictor.predict(str(image_path))

            # クラス名を正規化（出来型ボード系）
            for bbox in bboxes:
                cname = bbox.get('cname', '')
                if any(
                    keyword in cname
                    for keyword in ['出来形', '出来型', 'caption_board', 'dekigata']
                ):
                    bbox['cname'] = 'caption_board'
                    bbox['role'] = 'caption_board'

            # ボードサイズ判定
            is_closeup, ratio = self.classifier(
                bboxes,
                img_w,
                img_h,
                threshold_closeup=0.3,
                threshold_kanrichi=0.8,
                image_path=str(image_path),
            )

            return (is_closeup, ratio if ratio else 0.0)

        except Exception as e:
            print(f"  [ERROR] {image_path.name}: {e}")
            return (None, 0.0)

    @staticmethod
    def classify_board_size(ratio: float) -> str:
        """
        ボード面積比率からサイズ分類を判定

        Args:
            ratio: ボード面積比率 (0.0-1.0)

        Returns:
            "大" (0.8以上), "中" (0.5-0.8未満), "小" (0.3-0.5未満), "微小" (0.3未満)
        """
        if ratio >= 0.8:
            return "大"
        elif ratio >= 0.5:
            return "中"
        elif ratio >= 0.3:
            return "小"
        else:
            return "微小"

    def scan_folder(
        self, folder: Path, threshold: float = 0.3, size_filter: Optional[str] = None
    ) -> List[Dict]:
        """
        フォルダ内のボード写真を検出

        Args:
            folder: 対象フォルダ
            threshold: ボード面積比率の閾値（これ以上をボード写真とする）
            size_filter: サイズフィルタ（"大"/"中"/"小"/"微小"）

        Returns:
            ボード写真のリスト
        """
        images = self.get_image_files(folder)

        if not images:
            return []

        board_photos = []

        for img_path in images:
            is_closeup, ratio = self.detect_board_in_image(img_path)

            if ratio >= threshold:
                size_class = self.classify_board_size(ratio)

                # サイズフィルタ適用
                if size_filter and size_class != size_filter:
                    continue

                board_photos.append(
                    {
                        'path': str(img_path),
                        'relative_path': str(img_path.relative_to(folder)),
                        'filename': img_path.name,
                        'board_ratio': ratio,
                        'board_size': size_class,
                        'size': img_path.stat().st_size,
                    }
                )

                print(f"  ✓ {img_path.name} - ボードサイズ:{size_class} (比率: {ratio:.4f})")

        return board_photos

    def collect_from_folders(
        self,
        base_dir: Path,
        folder_pattern: str = "*",
        threshold: float = 0.8,
        size_filter: Optional[str] = None,
    ) -> Dict:
        """
        複数フォルダからボード写真を収集

        Args:
            base_dir: ベースディレクトリ
            folder_pattern: フォルダ検索パターン（例: "10*"）
            threshold: ボード面積比率の閾値
            size_filter: サイズフィルタ

        Returns:
            収集結果の辞書
        """
        target_folders = [
            d for d in base_dir.iterdir() if d.is_dir() and d.match(folder_pattern)
        ]

        print(f"検出された対象フォルダ: {len(target_folders)}個\n")

        all_board_photos = {}
        total_boards = 0

        for folder in sorted(target_folders):
            print(f"【{folder.name}】")
            board_photos = self.scan_folder(folder, threshold, size_filter)

            if board_photos:
                all_board_photos[folder.name] = {
                    'folder_path': str(folder),
                    'board_count': len(board_photos),
                    'photos': board_photos,
                }
                total_boards += len(board_photos)
                print(f"  -> {len(board_photos)}枚のボード写真を検出\n")
            else:
                print(f"  -> ボード写真なし\n")

        return {
            'base_directory': str(base_dir),
            'folder_pattern': folder_pattern,
            'threshold': threshold,
            'size_filter': size_filter,
            'total_folders': len(target_folders),
            'folders_with_boards': len(all_board_photos),
            'total_board_photos': total_boards,
            'folders': all_board_photos,
        }

    @staticmethod
    def calculate_file_hash(file_path: Path) -> str:
        """ファイルのMD5ハッシュを計算"""
        hash_md5 = hashlib.md5()
        with open(file_path, "rb") as f:
            for chunk in iter(lambda: f.read(4096), b""):
                hash_md5.update(chunk)
        return hash_md5.hexdigest()

    def deduplicate_by_hash(self, collection_result: Dict) -> Dict:
        """
        収集結果からハッシュで重複を除去

        Args:
            collection_result: collect_from_foldersの結果

        Returns:
            重複除去後の結果
        """
        print("\n=== ハッシュによる重複除去 ===\n")

        seen_hashes: Dict[str, Dict] = {}
        duplicates_count = 0

        for folder_name, folder_data in list(collection_result['folders'].items()):
            print(f"【{folder_name}】: {folder_data['board_count']}枚")

            unique_photos = []

            for photo in folder_data['photos']:
                file_path = Path(photo['path'])

                if not file_path.exists():
                    print(f"  [WARN] ファイルが存在しません: {file_path.name}")
                    continue

                # ハッシュ計算
                file_hash = self.calculate_file_hash(file_path)

                if file_hash in seen_hashes:
                    # 重複
                    original = seen_hashes[file_hash]
                    duplicates_count += 1
                    print(
                        f"  [DUP] {photo['filename']} (元: {original['folder']}/{original['filename']})"
                    )
                else:
                    # 新規
                    seen_hashes[file_hash] = {
                        'filename': photo['filename'],
                        'folder': folder_name,
                        'data': photo,
                    }
                    unique_photos.append(photo)

            folder_data['photos'] = unique_photos
            folder_data['board_count'] = len(unique_photos)

            print(f"  -> 重複除去後: {len(unique_photos)}枚\n")

        # 空のフォルダを削除
        collection_result['folders'] = {
            k: v for k, v in collection_result['folders'].items() if v['board_count'] > 0
        }

        # 合計を更新
        collection_result['total_board_photos'] = sum(
            v['board_count'] for v in collection_result['folders'].values()
        )
        collection_result['folders_with_boards'] = len(collection_result['folders'])
        collection_result['duplicates_removed'] = duplicates_count

        print(f"重複除去数: {duplicates_count}枚")
        print(f"最終合計: {collection_result['total_board_photos']}枚\n")

        return collection_result

    @staticmethod
    def save_to_json(result: Dict, output_path: Path):
        """結果をJSONファイルに保存"""
        with open(output_path, 'w', encoding='utf-8') as f:
            json.dump(result, f, ensure_ascii=False, indent=2)
        print(f"結果保存: {output_path}")

    @staticmethod
    def load_from_json(input_path: Path) -> Dict:
        """JSONファイルから結果を読み込み"""
        with open(input_path, encoding='utf-8') as f:
            return json.load(f)


def create_collector(summarygenerator_root: Path):
    """
    BoardPhotoCollectorインスタンスを作成

    Args:
        summarygenerator_root: summarygeneratorプロジェクトのルートパス

    Returns:
        BoardPhotoCollectorインスタンス
    """
    sys.path.insert(0, str(summarygenerator_root))

    from src.yolo_predict_core import YOLOPredictor
    from src.utils.caption_board_utils import judge_caption_board_closeup

    # YOLOモデルのパス（db_trainingモデルを使用）
    model_path = summarygenerator_root / "runs" / "train" / "db_training" / "weights" / "best.pt"

    predictor = YOLOPredictor(
        model_path=str(model_path), conf_threshold=0.25, iou_threshold=0.45
    )

    return BoardPhotoCollector(predictor, judge_caption_board_closeup)


# 使用例
if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description='ボード写真収集（汎用版）')
    parser.add_argument('base_dir', help='ベースディレクトリ')
    parser.add_argument(
        '--pattern', default='*', help='フォルダ検索パターン（例: 10*）'
    )
    parser.add_argument(
        '--threshold',
        type=float,
        default=0.8,
        help='ボード面積比率の閾値（デフォルト: 0.8）',
    )
    parser.add_argument('--size', choices=['大', '中', '小', '微小'], help='サイズフィルタ')
    parser.add_argument('--output', default='board_photos.json', help='出力JSONファイル')
    parser.add_argument(
        '--no-dedup', action='store_true', help='重複除去をスキップ'
    )

    args = parser.parse_args()

    # コレクター作成
    summarygenerator_root = Path(
        r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator"
    )
    collector = create_collector(summarygenerator_root)

    # 収集
    base_dir = Path(args.base_dir)
    result = collector.collect_from_folders(
        base_dir, args.pattern, args.threshold, args.size
    )

    # 重複除去
    if not args.no_dedup:
        result = collector.deduplicate_by_hash(result)

    # 保存
    output_path = Path(args.output)
    collector.save_to_json(result, output_path)

    print("\n=== 完了 ===")
    print(f"合計: {result['total_board_photos']}枚")
