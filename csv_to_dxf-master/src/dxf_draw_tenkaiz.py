from ezdxf.enums import TextEntityAlignment

def draw_road_sections(modelspace, data, scale=1000.0, text_height=350.0):
    """道路断面と測点ラベルを描画する
    
    Args:
        modelspace: DXFのモデル空間
        data: 測点データ（name, x, wl, wrカラムを持つDataFrame）
        scale: 図形のスケール倍率（デフォルト1000倍）
        text_height: テキストの高さ（幅員表示用、デフォルト350.0）
    """
    # テキストスタイルをMSPゴシックに設定
    dwg = modelspace.doc
    
    # コードページを日本語に設定（直接header変数を更新）
    try:
        dwg.header['$DWGCODEPAGE'] = 'ANSI_932'
        # ヘッダが更新されたか確認
        if dwg.header['$DWGCODEPAGE'] != 'ANSI_932':
            # バックアップ手段：別の方法でヘッダを設定
            if hasattr(dwg.header, 'set'):
                dwg.header.set('$DWGCODEPAGE', 'ANSI_932')
    except Exception as e:
        print(f"ヘッダー設定エラー: {e}")
    
    # MSPゴシックのスタイル設定
    if 'ＭＳ Ｐゴシック' not in dwg.styles:
        style = dwg.styles.new('ＭＳ Ｐゴシック', dxfattribs={'font': 'msgothic.ttc'})
    
    # 特殊スタイル（STYLE1）の設定
    if 'STYLE1' not in dwg.styles:
        style1 = dwg.styles.new('STYLE1', dxfattribs={'font': 'msgothic.ttc'})
        # 英語表記も設定（xdataを使用）
        try:
            # ezdxfのバージョンによって実装が異なるため、複数の方法を試す
            if hasattr(style1, 'set_xdata'):
                style1.set_xdata('ACAD', [('1000', 'MS PGothic')])
            elif hasattr(style1, 'set_app_data'):
                style1.set_app_data('ACAD', {'1000': 'MS PGothic'})
            elif hasattr(style1, 'set_extended_dict'):
                appdata = style1.get_extension_dict()
                if appdata:
                    appdata.add_dictionary('ACAD')
                    appdata['ACAD'].add_text('MS PGothic')
        except Exception as e:
            # エラーが発生しても処理を続行
            pass
    
    # ヘッダ変数でデフォルトのテキストスタイルを設定する
    try:
        dwg.header['$TEXTSTYLE'] = 'STYLE1'
        # 設定が反映されたか確認
        if dwg.header['$TEXTSTYLE'] != 'STYLE1':
            # バックアップ手段：別の方法でヘッダを設定
            if hasattr(dwg.header, 'set'):
                dwg.header.set('$TEXTSTYLE', 'STYLE1')
    except Exception as e:
        print(f"テキストスタイル設定エラー: {e}")
    
    prev_linelr = ((0,0),(0,0),(0,0))
    for index, row in data.iterrows():
        name, x, wl, wr = row['name'], row['x'], row['wl'], row['wr']
        # スケール調整
        x_scaled = x * scale
        wl_scaled = wl * scale
        wr_scaled = wr * scale
        
        linelr = ((x_scaled, wl_scaled), (x_scaled, 0), (x_scaled, -wr_scaled))

        line_conditions = coordinate_lines(row, prev_linelr, scale)
        dim_conditions, underline_conditions = coordinate_dimensions(row, prev_linelr, scale, text_height)

        draw_with(modelspace, line_conditions, draw_line)
        draw_with(modelspace, dim_conditions, draw_dim)
        draw_with(modelspace, underline_conditions, draw_line)

        prev_linelr = linelr

def coordinate_lines(row, prev_linelr, scale=1000.0):
    """線の描画条件を計算する
    
    Args:
        row: 測点データの行
        prev_linelr: 前の測点の左右ライン座標
        scale: スケール倍率
    """
    name, x, wl, wr = row['name'], row['x'], row['wl'], row['wr']
    # スケール調整
    x_scaled = x * scale
    wl_scaled = wl * scale
    wr_scaled = wr * scale
    
    linel = ((x_scaled, wl_scaled), (x_scaled, 0))
    liner = ((x_scaled, 0), (x_scaled, -wr_scaled))
    linec = ((x_scaled, 0), prev_linelr[1])
    linet = ((x_scaled, wl_scaled), prev_linelr[0])
    lineb = ((x_scaled, -wr_scaled), prev_linelr[2])
    conditions = [
        (True, linel),  # 幅員の線1を描画
        (True, liner),  # 幅員の線2を描画
        (linec[0][0] - linec[1][0] > 0, linec),  # センターラインを描画
        (linet[0][1] > 0, linet),  # 外形線1を描画
        (lineb[0][1] < 0, lineb)  # 外形線2を描画
    ]
    return conditions

def coordinate_dimensions(row, prev_points, scale=1000.0, text_height=350.0):
    """寸法の描画条件を計算する
    
    Args:
        row: 測点データの行
        prev_points: 前の測点の座標
        scale: スケール倍率
        text_height: 幅員表示用テキストの高さ
        
    Returns:
        tuple: (寸法条件リスト, アンダーライン条件リスト)
    """
    name, x, wl, wr = row['name'], row['x'], row['wl'], row['wr']
    # スケール調整
    x_scaled = x * scale
    wl_scaled = wl * scale
    wr_scaled = wr * scale
    
    prev_x = prev_points[1][0]
    prev_wl = prev_points[0][1]
    prev_wr = -prev_points[2][1]
    tankyori = x_scaled - prev_x
    alignment = align_by_distance(tankyori)
    
    # テキスト位置のオフセット調整
    text_offset = 500  # 幅員線の外側端からのオフセット距離
    
    # 延長寸法テキスト - 中央に配置（Y=0の位置）
    dimc = ('{:.2f}'.format(x - prev_x / scale), ((x_scaled + prev_x)*0.5, 0), 0, alignment, text_height)
    
    # 左側幅員テキスト - 幅員線の上端（外側端）からオフセットした位置に配置
    wl_text_pos_y = wl_scaled + text_offset  # 常に上端から一定距離
    diml = (f"{wl:.2f}", (x_scaled, wl_text_pos_y), -90, alignment, text_height)
    
    # 右側幅員テキスト - 幅員線の下端（外側端）からオフセットした位置に配置
    wr_text_pos_y = -wr_scaled - text_offset  # 常に下端から一定距離
    dimr = (f"{wr:.2f}", (x_scaled, wr_text_pos_y), -90, alignment, text_height)
    
    # 測点名テキスト - 回転-90度、幅員線上端基準のオフセット、青色に設定
    # 幅員線の上端からのオフセット計算
    if wl > 0:
        # 幅員線がある場合、その上端から一定距離
        text_vertical_offset = wl_scaled + 2000  # 上端から2000単位上
    else:
        # 幅員線がない場合、Y=0から一定距離
        text_vertical_offset = 2000  # Y=0から2000単位上
    
    # テキスト配置をBOTTOM_CENTERに変更（-90度回転テキストの右中央配置）
    dims = (name, (x_scaled, text_vertical_offset), -90, BOTTOM_CENTER, text_height, 5)  # カラーコード5（青色）
    
    conditions = [
        (tankyori > 0, dimc),  # 延長寸法を描画
        (wl > 0.0 and (x != prev_x / scale or wl != prev_wl / scale), diml),  # 左側の幅員寸法を描画
        (wr > 0.0 and (x != prev_x / scale or wr != prev_wr / scale), dimr),  # 右側の幅員寸法を描画
        ((x == 0 or x - prev_x / scale > 0), dims)  # 測点
    ]
    
    # アンダーライン条件のリスト
    underline_conditions = []
    
    # 測点テキスト用アンダーライン - 測点名が表示される場合のみ追加
    if (x == 0 or x - prev_x / scale > 0):
        # テキストの文字数からピッタリのライン長を計算
        char_count = len(name)
        
        # テキスト高さに文字数をかけて、ピッタリのライン長を計算（調整係数0.75）
        text_length = char_count * text_height * 0.75
        
        # 測点テキストの位置に合わせてアンダーラインの位置を調整（垂直方向）
        line_start = (x_scaled, text_vertical_offset - text_length/2)
        line_end = (x_scaled, text_vertical_offset + text_length/2)
        
        # アンダーライン描画情報を追加（'underline'マーカー、始点、終点、色コード）
        underline_conditions.append((True, ('underline', line_start, line_end, 5)))
    
    return conditions, underline_conditions

def draw_with(msp, conditions, drawmethod):
    """描画条件に基づいて描画メソッドを実行する"""
    for condition, entity in conditions:
        if condition:
            drawmethod(msp, entity)

def draw_line(msp, line):
    """線を描画する"""
    # アンダーライン描画（'underline'マーカー、始点、終点、色コードの4要素タプル）
    if isinstance(line, tuple) and len(line) == 4 and line[0] == 'underline':
        _, start, end, color = line
        dxf_line = msp.add_line(start, end)
        if color is not None:
            dxf_line.dxf.color = color
    # 通常の線描画（始点と終点の2点タプル）
    else:
        msp.add_line(line[0], line[1])

def draw_dim(msp, dim):
    """寸法を描画する"""
    # 新しい形式（6要素）と古い形式（5要素）の両方に対応
    if len(dim) == 6:
        text, position, rotation, alignment, text_height, color = dim
    else:
        text, position, rotation, alignment, text_height = dim
        color = None
        
    add_text(msp, text, position, rotation, alignment, text_height, color)

TOP_CENTER = TextEntityAlignment.TOP_CENTER
BOTTOM_CENTER = TextEntityAlignment.BOTTOM_CENTER
MIDDLE_CENTER = TextEntityAlignment.MIDDLE_CENTER

def add_text(msp, text, position, rotation=0, alignment=TOP_CENTER, text_height=350.0, color=None):
    """寸法テキストを追加する
    
    Args:
        msp: モデル空間
        text: テキスト内容
        position: テキスト位置
        rotation: 回転角度
        alignment: テキスト配置
        text_height: テキストの高さ
        color: テキストの色コード（指定しない場合はデフォルト）
    """
    # テキストの属性を設定（STYLE1を使用）
    dxfattrs = {'height': text_height, 'rotation': rotation, 'style': 'STYLE1'}
    
    # 色が指定されている場合は追加
    if color is not None:
        dxfattrs['color'] = color
    
    dimension_text = msp.add_text(text, dxfattribs=dxfattrs)
    dimension_text.dxf.insert = position
    dimension_text.dxf.align_point = position
    dimension_text.set_placement(position, align=alignment)

def align_by_distance(tankyori):
    """距離に基づいてテキスト配置を決定する"""
    if tankyori < 1000:  # スケール調整済みの値で比較
        return BOTTOM_CENTER
    else:
        return TOP_CENTER

