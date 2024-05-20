from ezdxf.enums import TextEntityAlignment

def draw_road_sections( modelspace, data ):
    """道路断面と測点ラベルを描画する"""
    prev_linelr = ((0,0),(0,0),(0,0))
    for index, row in data.iterrows():
        name, x, wl, wr = row['name'], row['x'], row['wl'], row['wr']
        linelr = ( (x,wl),(x,0),(x,-wr) )

        line_conditions = coodinate_lines( row, prev_linelr )
        dim_conditions = coodinate_dimensions( row, prev_linelr )

        draw_with( modelspace, line_conditions, draw_line )
        draw_with( modelspace, dim_conditions, draw_dim )

        prev_linelr = linelr

def coodinate_lines(row, prev_linelr):
    name, x, wl, wr = row['name'], row['x'], row['wl'], row['wr']
    linel = ((x, wl),  (x, 0))
    liner = ((x, 0),   (x, -wr))
    linec = ((x, 0),   prev_linelr[1])
    linet = ((x, wl),  prev_linelr[0])
    lineb = ((x, -wr), prev_linelr[2])
    conditions = [
        (True, linel),  # 幅員の線1を描画
        (True, liner),  # 幅員の線2を描画
        (linec[0][0] - linec[1][0] > 0, linec),  # センターラインを描画
        (linet[0][1] > 0, linet),  # 外形線1を描画
        (lineb[0][1] < 0, lineb)  # 外形線2を描画
    ]
    return conditions

def coodinate_dimensions(row, prev_points):
    name, x, wl, wr = row['name'], row['x'], row['wl'], row['wr']
    prev_x = prev_points[1][0]
    tankyori = x - prev_x
    alignment = align_by_distance(tankyori)
    dimc = ( '{:.2f}'.format(tankyori), ((x+prev_x)*0.5, 0), 0, alignment )
    diml = ( f"{wl:.2f}", (x, wl * 0.5), -90, alignment )
    dimr = ( f"{wr:.2f}", (x, -wr * 0.5), -90, alignment )
    dims = ( name, (x, wl + 5), -90, alignment )
    conditions = [
        (tankyori > 0, dimc),  # 延長寸法を描画
        (wl > 0.0, diml ),  # 左側の幅員寸法を描画
        (wr > 0.0, dimr ),  # 右側の幅員寸法を描画
        ( (x == 0 or x - prev_x > 0), dims ) #測点
    ]
    return conditions

def draw_with(msp, conditions, drawmethod ):
    for condition, entity in conditions:
        if condition:
            drawmethod(msp, entity)
def draw_line(msp, line):
    msp.add_line( line[0], line[1] )

def draw_dim(msp, dim):
    add_text(msp, dim[0], dim[1], dim[2], dim[3] )

TOP_CENTER=TextEntityAlignment.TOP_CENTER
BOTTOM_CENTER=TextEntityAlignment.BOTTOM_CENTER

def add_text(msp, text, position, rotation=0, alignment=TOP_CENTER):
    """寸法テキストを追加する"""
    dimension_text = msp.add_text(text, dxfattribs={'height': 1.0, 'rotation': rotation})
    dimension_text.dxf.insert = position
    dimension_text.dxf.align_point = position
    dimension_text.set_placement(position, align=alignment)

def align_by_distance(tankyori):
    if tankyori < 1:
        return BOTTOM_CENTER
    else:
        return TOP_CENTER

