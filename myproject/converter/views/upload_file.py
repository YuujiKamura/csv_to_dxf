# converter/views/upload_file.py

from django.shortcuts import render, redirect
from converter.forms import UploadFileForm  # 修正点
from converter.views.handle_upload import handle_uploaded_file  # 修正点

def upload_file(request):
    if request.method == 'POST':
        form = UploadFileForm(request.POST, request.FILES)
        if form.is_valid():
            dxf_path = handle_uploaded_file(request.FILES['file'])
            return render(request, 'converter/result.html')
    else:
        form = UploadFileForm()
    return render(request, 'converter/upload.html', {'form': form})
