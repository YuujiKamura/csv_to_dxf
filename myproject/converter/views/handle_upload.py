# converter/views/handle_upload.py

def handle_uploaded_file(file):
    dxf_path = 'uploaded_file.dxf'
    with open(dxf_path, 'wb+') as destination:
        for chunk in file.chunks():
            destination.write(chunk)
    return dxf_path
