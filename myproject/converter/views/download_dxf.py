# converter/views/download_dxf.py

import os
from django.http import FileResponse, HttpResponse

def download_dxf(request):
    dxf_path = 'uploaded_file.dxf'
    if os.path.exists(dxf_path):
        return FileResponse(open(dxf_path, 'rb'), as_attachment=True, filename='output.dxf')
    else:
        return HttpResponse("DXF file not found.")
