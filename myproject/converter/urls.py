# converter/urls.py

from django.urls import path
from .views import upload_file, download_dxf, show_pdf

urlpatterns = [
    path('upload/', upload_file, name='upload_file'),
    path('download_dxf/', download_dxf, name='download_dxf'),
    path('show_pdf/', show_pdf, name='show_pdf'),
]
