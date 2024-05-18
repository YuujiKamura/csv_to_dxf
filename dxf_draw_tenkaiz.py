from ezdxf.enums import TextEntityAlignment


def draw_road_sections(msp, data):
    """道路断面と測点ラベルを描画する"""
    prev_points = (None,None,None)

    for index, row in data.iterrows():
        name, x, wl, wr = row['name'], row['x'], row['wl'], row['wr']
        left = (x, wl)
        center = (x, 0)
        right = (x, -wr)

        draw_matomete_lines(left, center, right, prev_points, msp)
        draw_set_of_dimensions(data, index, msp, row)

        prev_points = ( left, center, right )

def draw_matomete_lines(left_point, center_point, right_point, prev_points, msp):
    draw_hukuin_lines(center_point, left_point, msp, right_point)
    draw_gaikeisen(left_point, msp, prev_points[0], prev_points[2], right_point)
    draw_centerline(center_point, msp, prev_points[1])

def draw_hukuin_lines(center_point, left_point, msp, right_point):
    # 幅員の線を描画
    msp.add_line(left_point, center_point)
    msp.add_line(center_point, right_point)

def draw_centerline(center_point, msp, previous_center_point):
    # センターラインを描画
    if previous_center_point:
        msp.add_line(previous_center_point, center_point)

def draw_gaikeisen(left_point, msp, previous_left_point, previous_right_point, right_point):
    # 外形線を描画
    if previous_left_point and previous_right_point:
        if left_point[1] > 0:
            msp.add_line(previous_left_point, left_point)
        if right_point[1] < 0:
            msp.add_line(previous_right_point, right_point)

def draw_set_of_dimensions(data, index, msp, row):
    name, x, wl, wr = row['name'], row['x'], row['wl'], row['wr']
    prev_x = 0
    if index > 0:
        prev_x = data.iloc[index - 1]['x']
    # 延長寸法を描画
    if x - prev_x > 0.0:
        add_text(msp, f"{x - prev_x:.2f}", ((x + prev_x) / 2, 0))
    if x - prev_x > 1.0:
        draw_sokuten(msp, name, wl, wr, x)
    alignment = TOP_CENTER
    if x - prev_x < 1.0:
        alignment = BOTTOM_CENTER
    # 左側の幅員寸法を描画
    draw_positive_dimension(msp, wl, (x, wl / 2), -90, alignment)
    # 右側の幅員寸法を描画
    draw_positive_dimension(msp, wr, (x, -wr / 2), -90, alignment)

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

def draw_sokuten(msp, name, wl, wr, x, alignment=TOP_CENTER):
    # 測点ラベルを追加、-90度回転して上側に配置
    add_text(msp, name, (x, max(wl, -wr) + 5), -90, alignment)
