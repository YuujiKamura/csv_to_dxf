def convert_dxf_to_pdf(dxf_path, scale=2):
    # DXFファイルを読み込み
    doc = ezdxf.readfile(dxf_path)
    msp = doc.modelspace()

    # 図のサイズを2倍に設定
    fig, ax = plt.subplots(figsize=(8 * scale, 6 * scale))  # 例えば元のサイズが (8, 6) だった場合

    ax.set_aspect('equal')
    ax.axis('off')

    for entity in msp:
        if entity.dxftype() == 'LINE':
            start = entity.dxf.start
            end = entity.dxf.end
            ax.plot([start[0], end[0]], [start[1], end[1]], 'k-')
        elif entity.dxftype() == 'TEXT':
            pos = entity.dxf.insert
            rotation = entity.dxf.rotation
            if rotation == 0:
                ha = 'center'
                va = 'bottom'
            else:
                ha = 'right'
                va = 'center'
            ax.text(pos[0], pos[1], entity.dxf.text, fontsize=4 * scale, rotation=entity.dxf.rotation, ha=ha, va=va)

    buffer = io.BytesIO()
    plt.savefig(buffer, format='pdf')
    buffer.seek(0)
    plt.close(fig)  # 図を閉じてメモリを解放
    return buffer