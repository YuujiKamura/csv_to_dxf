import os
import json

class ConfigManager:
    """アプリケーションの設定を管理するクラス"""
    
    DEFAULT_CONFIG = {
        "last_directory": "",
        "auto_convert": True,
        "normalize_station_names": True,
        "special_sections": {
            "区間6": {
                "swap_sides": True
            }
        },
        "output_directory": "",
        "recent_files": [],
        "max_recent_files": 5,
        "default_dxf_version": "R2010"
    }
    
    def __init__(self, config_file="app_config.json"):
        """
        初期化処理
        
        Args:
            config_file: 設定ファイルのパス
        """
        self.config_file = config_file
        self.config = self.DEFAULT_CONFIG.copy()
        self.load_config()
    
    def load_config(self):
        """設定ファイルから設定を読み込む"""
        try:
            if os.path.exists(self.config_file):
                with open(self.config_file, 'r', encoding='utf-8') as f:
                    loaded_config = json.load(f)
                    # 既存の設定とマージ
                    self.config.update(loaded_config)
                return True
        except Exception as e:
            print(f"設定ファイル読み込みエラー: {e}")
        return False
    
    def save_config(self):
        """設定をファイルに保存する"""
        try:
            with open(self.config_file, 'w', encoding='utf-8') as f:
                json.dump(self.config, f, ensure_ascii=False, indent=2)
            return True
        except Exception as e:
            print(f"設定ファイル保存エラー: {e}")
            return False
    
    def get(self, key, default=None):
        """
        設定値を取得する
        
        Args:
            key: 設定キー
            default: デフォルト値
            
        Returns:
            設定値
        """
        return self.config.get(key, default)
    
    def set(self, key, value):
        """
        設定値を設定する
        
        Args:
            key: 設定キー
            value: 設定値
            
        Returns:
            bool: 成功したかどうか
        """
        self.config[key] = value
        return self.save_config()
    
    def add_recent_file(self, file_path):
        """
        最近使用したファイルリストに追加する
        
        Args:
            file_path: ファイルパス
            
        Returns:
            bool: 成功したかどうか
        """
        if not file_path:
            return False
            
        # 既存のリストを取得
        recent_files = self.get("recent_files", [])
        
        # すでにリストにある場合は削除して先頭に追加
        if file_path in recent_files:
            recent_files.remove(file_path)
        
        # 先頭に追加
        recent_files.insert(0, file_path)
        
        # 最大数に制限
        max_files = self.get("max_recent_files", 5)
        if len(recent_files) > max_files:
            recent_files = recent_files[:max_files]
        
        # 設定を更新
        self.set("recent_files", recent_files)
        
        # 最後に使用したディレクトリも更新
        last_dir = os.path.dirname(file_path)
        if last_dir:
            self.set("last_directory", last_dir)
        
        return True
    
    def get_section_config(self, section_name):
        """
        特定の区間の設定を取得する
        
        Args:
            section_name: 区間名
            
        Returns:
            dict: 区間の設定
        """
        special_sections = self.get("special_sections", {})
        return special_sections.get(section_name, {}) 