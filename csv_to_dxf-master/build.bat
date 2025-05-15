@echo off
echo === Nuitkaビルド ===

nuitka main.py ^
  --standalone ^
  --onefile ^
  --enable-plugin=pyqt6 ^
  --windows-icon-from-ico=csv_to_dxf_gi.ico ^
  --windows-uac-admin ^
  --windows-company-name="YourName" ^
  --windows-product-name="CSV to DXF" ^
  --windows-product-version="1.0.0" ^
  --disable-console

echo === ビルド完了 ===
pause
