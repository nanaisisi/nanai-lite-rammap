# nanai-lite-rammap

AI生成による軽量メモリ・GPUリソース可視化ツールです。  
タスクマネージャー程度の低負荷で動作する、RAMMapのLite版的立ち位置として作成されています。

GUIフレームワークには、Microsoft公式系のアプローチに近い **windows-reactor** を採用しています。

---

## 🌟 主な特徴

- **2次元ツリーマップ表示 (WizTreeスタイル)**
  - プロセスごとのメモリ／VRAM使用量を面積比率として一目で直感的に把握可能。
- **システムRAM & GPU VRAMのデュアル監視**
  - **System RAM**: 物理メモリ (Working Set / Private Commit) の内訳とシステム全体のコミット状況。
  - **GPU VRAM (DXGI + D3DKMT)**: 各GPUアダプターごとの専用ビデオメモリ (Dedicated VRAM) および共有GPUメモリ (Shared Memory)、プロセス別のGPUメモリ消費をリアルタイム取得。
- **多彩な集計・表示モード**
  - **グループ化**: 「プロセス名ごと (ByName)」「カテゴリごと (ByCategory: ブラウザ/開発/ゲーム等)」「個別プロセス (Individual)」
  - **ビューモード**: 「2D Treemap表示」 / 「詳細テーブルリスト表示」
  - **フィルタリング**: PID・プロセス名・カテゴリ名による高速インクリメンタル検索。

---

## 🛠️ 開発・ビルドコマンド

```bash
# 実行
cargo run

# Clippyチェック & 自動修正
cargo clippy --fix --all --allow-dirty

# 依存関係の脆弱性チェック
cargo audit fix
```
