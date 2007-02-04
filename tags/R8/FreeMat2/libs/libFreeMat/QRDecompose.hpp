// Copyright (c) 2004 Samit Basu
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

#ifndef __QRDecompose_hpp__
#define __QRDecompose_hpp__

namespace FreeMat {
  void floatQRD(const int nrows, const int ncols, float *q, float *r, float *a);
  void doubleQRD(const int nrows, const int ncols, double *q, double *r, double *a);
  void complexQRD(const int nrows, const int ncols, float *q, float *r, float *a);
  void dcomplexQRD(const int nrows, const int ncols, double *q, double *r, double *a);
  void floatQRDP(const int nrows, const int ncols, float *q, float *r, int *p, float *a);
  void doubleQRDP(const int nrows, const int ncols, double *q, double *r, int *p, double *a);
  void complexQRDP(const int nrows, const int ncols, float *q, float *r, int *p, float *a);
  void dcomplexQRDP(const int nrows, const int ncols, double *q, double *r, int *p, double *a);
}

#endif