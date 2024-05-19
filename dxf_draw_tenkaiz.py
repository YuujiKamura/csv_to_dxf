from ezdxf.enums import TextEntityAlignment

def draw_road_sections(msp, data):
    """道路断面と測点ラベルを描画する"""
    prev_points = (None,None,None)
    for index, row in data.iterrows():
        name, x, wl, wr = row['name'], row['x'], row['wl'], row['wr']
        points = ((x,wl),(x,0),(x,wr))
        draw_matomete_lines( msp, points, prev_points )
        draw_set_of_dimensions( msp, name, wl, wr, points, prev_points )
        prev_points = points

def draw_matomete_lines(msp, points, prev_points):
    draw_hukuin_lines(msp, points)
    draw_gaikeisen(msp, points, prev_points)
    draw_centerline(msp, points, prev_points)

def draw_hukuin_lines(msp, points):
    # 幅員の線を描画
    msp.add_line(points[0],points[1])
    msp.add_line(points[1],points[2])

def draw_centerline(msp,points,prev_points):
    # センターラインを描画
    if points[1][0]-prev_points[1][0] > 0:
        msp.add_line(points[1], prev_points[1])

def draw_gaikeisen(msp, points, prev_points):
    # 外形線を描画
    if prev_points[0] and prev_points[2]:
        if points[0][1] > 0:
            msp.add_line(prev_points[0], points[0])
        if points[2][1] < 0:
            msp.add_line(prev_points[2], points[2])

def draw_set_of_dimensions( msp, name, wl, wr, points, prev_points ):
    added_distance = points[1][0]
    prev_x = prev_points[1][0]
    alignment = align_by_distance(added_distance, prev_x)
    # 延長寸法を描画
    if added_distance - prev_x > 1.0:
        add_text(msp, f"{added_distance - prev_x:.2f}", ((added_distance + prev_x) / 2, 0))

    draw_sokuten(msp, name, wl, wr, added_distance, prev_x, alignment)

    # 左側の幅員寸法を描画
    draw_positive_dimension(msp, wl, (added_distance, wl * 0.5 ), -90, alignment)
    # 右側の幅員寸法を描画
    draw_positive_dimension(msp, wr, (added_distance, -wr * 0.5 ), -90, alignment)

def align_by_distance(x, prev_x):
    if x - prev_x < 1:
        return BOTTOM_CENTER
    else:
        return TOP_CENTER

TOP_CENTER=TextEntityAlignment.TOP_CENTER
BOTTOM_CENTER=TextEntityAlignment.BOTTOM_CENTER

def draw_positive_dimension(msp, value, position, rotation, alignment=TOP_CENTER):
    if value > 0.0:
        add_text(msp, f"{value:.2f}", position, rotation, alignment)

def add_text(msp, text, position, rotation=0, alignment=TOP_CENTER):
    """寸法テキストを追加する"""
    dimension_text = msp.add_text(text, dxfattribs={'height': 1.0, 'rotation': rotation})
    dimension_text.dxf.insert = position
    dimension_text.dxf.align_point = position
    dimension_text.set_placement(position, align=alignment)

def draw_sokuten(msp, name, wl, wr, x, prev_x, alignment=TOP_CENTER):
    if x - prev_x > 0:
        # 測点ラベルを追加、-90度回転して上側に配置
        add_text(msp, name, (x, max(wl, -wr) + 5), -90, alignment)
