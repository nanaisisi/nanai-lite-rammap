# winapp CLI / MSIX パッケージ手順

このリポジトリでは実行ファイル名が `nanai-lite-rammap.exe` です。
`winapp` および `build-msix.ps1` を使用したビルド・パッケージ化手順は以下の通りです。

---

## 🛠️ ワンコマンド実行 (`build-msix.ps1`)

### 1. デバッグ実行 (Debug Identity 付与 & 起動)

```powershell
.\build-msix.ps1 -Mode Debug
```

### 2. MSIX パッケージ生成 & インストール

```powershell
.\build-msix.ps1 -Mode Package
```

※ パッケージ化のみ行いたい場合（証明書・アプリの自動インストールをスキップ）：

```powershell
.\build-msix.ps1 -Mode Package -SkipCertInstall -SkipMsixInstall
```

---

## 📖 手動コマンド手順

### 1. デバッグ ID を使用した実行

1. デバッグビルドを行います：

   ```powershell
   cargo build
   ```

2. デバッグ ID を付与します：

   ```powershell
   winapp create-debug-identity .\target\debug\nanai-lite-rammap.exe
   ```

3. 実行します：

   ```powershell
   .\target\debug\nanai-lite-rammap.exe
   ```

---

### 2. MSIX を使ったパッケージ化

1. リリースビルドを行います：

   ```powershell
   cargo build --release
   ```

2. パッケージ用ディレクトリ `dist` を準備します：

   ```powershell
   mkdir dist
   Copy-Item .\target\release\nanai-lite-rammap.exe .\dist\
   Copy-Item .\target\release\*.dll .\dist\
   Copy-Item .\target\release\*.pri .\dist\ -ErrorAction SilentlyContinue
   Copy-Item .\Package.appxmanifest .\dist\
   Copy-Item .\Assets .\dist\ -Recurse
   ```

3. 開発用証明書を生成します（既存がある場合はスキップ）：

   ```powershell
   winapp cert generate --if-exists skip
   ```

4. パッケージ化して署名します：

   ```powershell
   winapp package .\dist --manifest .\dist\Package.appxmanifest --cert .\devcert.pfx --output nanai-lite-rammap.msix
   ```

5. 証明書をインストールします（管理者権限）：

   ```powershell
   winapp cert install .\devcert.pfx
   ```

6. MSIX をインストールします：

   ```powershell
   Add-AppxPackage .\nanai-lite-rammap.msix
   ```
