#!/bin/bash
# ファイル行数チェック - pre-push hook用
# 使用: ./scripts/check_file_size.sh [--warn-only] [--max=N] [--warn=N]

MAX_LINES=800    # 現状の最大ファイルに合わせて調整
WARN_LINES=500
WARN_ONLY=false

for arg in "$@"; do
    case $arg in
        --warn-only) WARN_ONLY=true ;;
        --max=*) MAX_LINES="${arg#*=}" ;;
        --warn=*) WARN_LINES="${arg#*=}" ;;
    esac
done

echo "=== File Size Check ==="
echo "Error: >${MAX_LINES} lines, Warning: >${WARN_LINES} lines"
[ "$WARN_ONLY" = true ] && echo "(warn-only mode)"
echo ""

ERRORS=$(find . -type d \( -name ".venv" -o -name "target" -o -name "target2" -o -name "node_modules" -o -name "__pycache__" -o -name ".git" \) -prune -o \( -name "*.py" -o -name "*.rs" \) -type f -print 2>/dev/null \
| xargs wc -l 2>/dev/null \
| grep -v " total$" \
| sort -rn \
| awk -v max="$MAX_LINES" -v warn="$WARN_LINES" '
    $1 > max  { print "ERROR: " $2 " (" $1 " lines)"; errors++ }
    $1 > warn && $1 <= max { print "WARN:  " $2 " (" $1 " lines)" }
    END {
        if (errors > 0) {
            print ""
            print "Found " errors " file(s) exceeding " max " lines."
            print errors
        } else {
            print "OK"
            print "0"
        }
    }
' | tee /dev/stderr | tail -1)

if [ "$WARN_ONLY" = true ]; then
    exit 0
fi

[ "$ERRORS" -gt 0 ] 2>/dev/null && exit 1
exit 0
