# converter/views/show_pdf.py

import io
import ezdxf
import matplotlib

matplotlib.use('Agg')
import matplotlib.pyplot as plt
from django.http import HttpResponse


def convert_dxf_to_pdf(dxf_path, scale=2):
    doc = ezdxf.readfile(dxf_path)
    msp = doc.modelspace()

    fig, ax = plt.subplots(figsize=(8 * scale, 6 * scale))

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

            ax.text(pos[0], pos[1], entity.dxf.text, fontsize=8 / scale, rotation=rotation, ha=ha, va=va)

    buffer = io.BytesIO()
    plt.savefig(buffer, format='pdf')
    buffer.seek(0)
    plt.close(fig)
    return buffer


def show_pdf(request):
    dxf_path = 'uploaded_file.dxf'
    pdf_buffer = convert_dxf_to_pdf(dxf_path)
    response = HttpResponse(pdf_buffer, content_type='application/pdf')
    response['Content-Disposition'] = 'inline; filename="output.pdf"'
    return response
