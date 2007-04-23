// Copyright (c) 2002, 2003 Samit Basu
// 
// Permission is hereby granted, free of charge, to any person obtaining a 
// copy of this software and associated documentation files (the "Software"), 
// to deal in the Software without restriction, including without limitation 
// the rights to use, copy, modify, merge, publish, distribute, sublicense, 
// and/or sell copies of the Software, and to permit persons to whom the 
// Software is furnished to do so, subject to the following conditions:
// 
// The above copyright notice and this permission notice shall be included 
// in all copies or substantial portions of the Software.
// 
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS 
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, 
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL 
// THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER 
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING 
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER 
// DEALINGS IN THE SOFTWARE.

#include "ScalarImage.hpp"
#include <math.h>
#include <iostream>
#include "Malloc.hpp"
#include "GraphicsCore.hpp"
#include "FL/Fl.H"

namespace FreeMat {

  ScalarImage::ScalarImage(int width, int height) : PrintableWidget(0,0,width,height) {
    rawData = NULL;
    for (int i=0;i<256;i++) {
      colormap[0][i] = i;
      colormap[1][i] = i;
      colormap[2][i] = i;
    }
    window = 0;
    level = 0;
    zoomImage = NULL;
    picData = NULL;
    inClickState = false;
  }
  
  ScalarImage::~ScalarImage() {
    if (rawData) Free(rawData);
  }


  int ScalarImage::handle(int event) {
    if (event == FL_PUSH) {
      inClickState = false;
      xposClick = Fl::event_x();
      yposClick = Fl::event_y();
      return 1;
    }
    return 0;
  }

  Array ScalarImage::GetPoint() {
    inClickState = true;
    ((Fl_Window*)parent())->cursor(FL_CURSOR_CROSS);
    while (inClickState)
      Fl::wait(0);
    ((Fl_Window*)parent())->cursor(FL_CURSOR_DEFAULT);
    double valClick;
    if (zoomImage == NULL) 
      valClick = atof("nan");
    else 
      valClick = zoomImage[xposClick*zoomRows+yposClick];
    Array retval(Array::doubleVectorConstructor(3));
    double *d_ip;
    d_ip = (double*) retval.getReadWriteDataPointer();
    d_ip[0] = (double) (yposClick/((double)zoomRows)*rows)+1;
    d_ip[1] = (double) (xposClick/((double)zoomColumns)*columns)+1;
    d_ip[2] = (double) valClick;
    return retval;
  }

  void ScalarImage::SetColormap(Array &dp) {
    dp.promoteType(FM_DOUBLE);
    double *dpdat;
    dpdat = (double*) dp.getDataPointer();
    for (int j=0;j<3;j++)
      for (int i=0;i<256;i++)
	colormap[j][i] = (char)(255.0*(*dpdat++));
    UpdateImage();
  }

  void ScalarImage::Zoom1D(double* src, double *dst,
		       int s_stride, int d_stride,
		       int s_count, int d_count) {
    double d_delta;
    d_delta = s_count/((double) d_count);
    for (int i=0;i<d_count;i++) {
      int srcPoint = (int) (i*d_delta);
      dst[i*d_stride] = src[srcPoint*s_stride];
    }
  }

  void ScalarImage::Zoom(float z) {
    zoom = z;
    UpdateZoom(false);
    //    if (z>0) SetSize(zoomColumns,zoomRows);
  }

  void ScalarImage::UpdateZoom(bool forceUpdate) {
    int newZoomRows, newZoomColumns;
	int i;
    if (rawData == NULL) return;
    if (zoom>0) {
      newZoomColumns = (int) (zoom*columns);
      newZoomRows = (int) (zoom*rows);
    } else if (zoom == 0) {
      int client_width;
      int client_height;
      client_width = w();
      client_height = h();
      double zoomColFactor;
      zoomColFactor = ((double) client_width)/columns;
      double zoomRowFactor;
      zoomRowFactor = ((double) client_height)/rows;
      double effectiveZoom;
      effectiveZoom = (zoomColFactor < zoomRowFactor) ? 
	zoomColFactor : zoomRowFactor;
      newZoomColumns = (int) (effectiveZoom*columns);
      newZoomRows = (int) (effectiveZoom*rows);
    } else {
      int client_width;
      int client_height;
      client_width = w();
      client_height = h();
      newZoomColumns = (int) (client_width);
      newZoomRows = (int) (client_height);
    }
    if (newZoomColumns == zoomColumns &&
	newZoomRows == zoomRows &&
	zoomImage != NULL &&
	!forceUpdate) return;
    zoomColumns = newZoomColumns;
    zoomRows = newZoomRows;
    if (zoomImage != NULL)
      delete[] zoomImage;
    if (zoomColumns < 1) zoomColumns = 1;
    if (zoomRows < 1) zoomRows = 1;
    if (zoomColumns > 3000) zoomColumns = 3000;
    if (zoomRows > 3000) zoomRows = 3000;
    zoomImage = new double[zoomColumns*zoomRows];
    double *tmpImage = new double[zoomRows*columns];
    // First zoom the columns
    for (i=0;i<columns;i++)
      Zoom1D(rawData + i*rows,tmpImage + i*zoomRows,
	     1,1,rows,zoomRows);
    // Then zoom the rows 
    for (i=0;i<zoomRows;i++)
      Zoom1D(tmpImage + i, zoomImage + i,
	     zoomRows, zoomRows,
	     columns, zoomColumns);
    delete[] tmpImage;
    //    SetSize(zoomColumns,zoomRows);
    //    SetSize(zoomColumns,zoomRows);
    UpdateImage();
  }

  void ScalarImage::draw() {
    FLTKGC gc(w(),h());
    OnDraw(gc);
  }

  void ScalarImage::resize(int x, int y, int w, int h) {
    Fl_Widget::resize(x,y,w,h);
    if (zoom <= 0) 
      UpdateZoom(false);
    redraw();
  }

  void ScalarImage::OnDraw(GraphicsContext &gc) {
    if (rawData == NULL) return;
    gc.BlitImage(picData, zoomColumns, zoomRows, 0, 0);
  }

  void ScalarImage::SetImageArray(Array &dp, double zoomf) {
    if (rawData != NULL)
      Free(rawData);
    rows = dp.getDimensionLength(0);
    columns = dp.getDimensionLength(1);
    rawData = (double*) Malloc(rows*columns*sizeof(double));
    memcpy(rawData,dp.getDataPointer(),rows*columns*sizeof(double));
    double minval;
    double maxval;
    minval = rawData[0];
    maxval = rawData[0];
    int length;
    length = rows*columns;
    for (int i=0;i<length;i++) {
      minval = (minval < rawData[i]) ? minval : rawData[i];
      maxval = (maxval > rawData[i]) ? maxval : rawData[i];
    }
    window = maxval - minval;
    level = (maxval + minval)/2.0;
    zoom = zoomf;
    UpdateZoom(true);
    Zoom(zoom);
  }

  void ScalarImage::WindowLevel(double awindow, double alevel) {
    window = awindow;
    level = alevel;
    UpdateImage();
  }

  double ScalarImage::GetCurrentWindow() {
    return window;
  }

  double ScalarImage::GetCurrentLevel() {
    return level;
  }

  void ScalarImage::UpdateImage() {
    if (rawData == NULL) return;
    if (picData != NULL)
      delete picData;
    picData = new byte[zoomColumns*zoomRows*3];
    byte *op;
    op = picData;
    int length = zoomColumns * zoomRows;
    double minval = level - window/2.0;
    double delta = 255.0/window;
    for (int i=0;i<zoomColumns;i++) {
      for (int j=0;j<zoomRows;j++) {
	int dv, ndx;
	dv = (int)((zoomImage[i*zoomRows+j] - minval)*delta);
	dv = (dv < 0) ? 0 : dv;
	dv = (dv > 255) ? 255 : dv;
	ndx = 3*(i+zoomColumns*j);
	op[ndx] = colormap[0][dv];
	op[ndx+1] = colormap[1][dv];
	op[ndx+2] = colormap[2][dv];
      }
    }
  }
}