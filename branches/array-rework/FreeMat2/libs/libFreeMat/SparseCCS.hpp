#ifndef __SparseCCS_hpp__
#define __SparseCCS_hpp__

#include <QVector>
#include "SparseMatrix.hpp"
#include "Array.hpp"

QVector<uint32> CompressCCSCols(const QVector<uint32> &cols, index_t colcount);

template <typename T>
void SparseToCCS(const SparseMatrix<T>&A,
		 QVector<uint32> &rowstart,
		 QVector<uint32> &colstart,
		 QVector<T> &Adata) {
  QVector<uint32> cols;
  ConstSparseIterator<T> iter(&A);
  while (iter.isValid()) {
    while (iter.moreInSlice()) {
      cols << uint32(iter.col()-1);
      rowstart << uint32(iter.row()-1);
      Adata << iter.value();
      iter.next();
    }
    iter.nextSlice();
  }
  colstart = CompressCCSCols(cols,A.cols());
}

template <typename T>
void SparseToCCS(const SparseMatrix<T> &Areal, 
		 const SparseMatrix<T> &Aimag,
		 QVector<uint32> &rowstart,
		 QVector<uint32> &colstart,
		 QVector<T> &Areal_part,
		 QVector<T> &Aimag_part) {
  QVector<uint32> cols;
  ConstComplexSparseIterator<T> iter(&Areal,&Aimag);
  while (iter.isValid()) {
    while (iter.moreInSlice()) {
      cols << uint32(iter.col()-1);
      rowstart << uint32(iter.row()-1);
      Areal_part << iter.realValue();
      Aimag_part << iter.imagValue();
    }
    iter.nextSlice();
  }
  colstart = CompressCCSCols(cols,Areal.cols());
}

Array IJVToSparse(const BasicArray<index_t> &ip,const BasicArray<index_t> &jp,const Array &dat);
Array IJVToSparse(const BasicArray<index_t> &ip, const BasicArray<index_t> &jp, const Array &dat,
		  index_t rows, index_t cols);

Array SparseToIJV(const Array &a, Array &rows, Array &cols);

template <class T>
class RLEEncoderComplex {
  QVector<T> buffer;
  int m;
  int zlen;
  int state;
public:
  RLEEncoderComplex() {
    m = 0;
    state = 0;
    zlen = 0;
  }
  int row() {
    return m;
  }
  void set(int p) {
    if (p <= m) return;
    p -= m;
    if (state == 0) {
      zlen = p;
      state = 1;
    } else
      zlen += p;
    m += p;  
  }
  void push(T valr, T vali) {
    if (state == 0) {
      if ((valr != 0) || (vali != 0)) {
	buffer << valr;
	buffer << vali;
      } else {
	state = 1;
	zlen = 1;
      }
    } else {
      if ((valr == 0) && (vali == 0)) {
	zlen++;
      } else {
	if (zlen) {
	  buffer << 0;
	  buffer << 0;
	  buffer << zlen;
	}
	buffer << valr;
	buffer << vali;
	state = 0;
	zlen = 0;
      }
    }
    m++;
  }
  void end(int len) {
    set(len);
    if (zlen>0) {
      buffer << 0;
      buffer << 0;
      buffer << zlen;
    }
    state = 0;
  }
  QVector<T> copyout() {
    return buffer;
  }
};


template <class T>
class RLEEncoder {
  QVector<T> buffer;
  int m;
  int zlen;
  int state;
public:
  RLEEncoder() {
    m = 0;
    state = 0;
    zlen = 0;
  }
  int row() {return m;}
  void set(int p) {
    if (p <= m) return;
    p -= m;
    if (state == 0) {
      zlen = p;
      state = 1;
    } else
      zlen += p;
    m += p;  
  }
  void push(T val) {
    if (state == 0) {
      if (val != 0) {
	buffer << val;
      } else {
	state = 1;
	zlen = 1;
      }
    } else {
      if (val == 0) {
	zlen++;
      } else {
	if (zlen) {
	  buffer << 0;
	  buffer << zlen;
	}
	buffer << val;
	state = 0;
	zlen = 0;
      }
    }
    m++;
  }
  void end(int len) {
    set(len);
    if (zlen) {
      buffer << 0;
      buffer << zlen;
    }
    state = 0;
  }
  QVector<T> copyout() {
    return buffer;
  }
};

template <class T>
class RLEDecoder {
  const QVector<T> &data;
  int m;
  int n;
  int len;
public:
  RLEDecoder(const QVector<T> &str) : data(str) {
    m = 0;
    n = 1;
    len = str.size();
  }
  int row() {
    return m;
  }
  void update() {
    while ((m < len) && (data[n] == 0)) {
      m += (int) data[n+1];
      n += 2;
      if ((m < len) && (n>((int)data[0]))) {
	throw Exception("Invalid data string!\n");
      }
    }
  }
  void advance() {
    if (m < len) {
      m++;
      n++;
      update();
    }
  }
  T value() {
    if (m >= len)
      throw Exception("RLE Decoder overflow - corrupted sparse matrix string encountered");
    return data[n];
  }
  bool more() {
    return (m < len);
  }
  int nnzs() {
    int nnz = 0;
    int t = 0;
    int p = 1;
    while (t < len) {
      if (data[p] != 0) {
	nnz++;
	p++;
	t++;
      } else {
	t += (int) data[p+1];
	p += 2;
      }
    }
    return nnz;
  }
};

template <class T>
class RLEDecoderComplex {
  const QVector<T>& data;
  int m;
  int n;
  int len;
public:
  RLEDecoderComplex(const QVector<T>& str) : data(str) {
    m = 0;
    n = 1;
    len = str.size();
  }
  int row() {
    return m;
  }
  void update() {
    while ((m < len) && (data[n] == 0) && (data[n+1] == 0)) {
      m += (int) data[n+2];
      n += 3;
    }
  }
  void advance() {
    if (m < len) {
      m++;
      n+=2;
      update();
    }
  }
  T value_real() {
    if (m >= len)
      throw Exception("RLE DecoderComplex overflow - corrupted sparse matrix string encountered");
    return data[n];
  }
  T value_imag() {
    if (m >= len)
      throw Exception("RLE DecoderComplex overflow - corrupted sparse matrix string encountered");
    return data[n+1];
  }
  bool more() {
    return (m < len);
  }
  int nnzs() {
    int nnz = 0;
    int t = 0;
    int p = 1;
    while (t < len) {
      if ((data[p] != 0) || (data[p+1] != 0)) {
	nnz++;
	p+=2;
	t++;
      } else {
	t += (int) data[p+2];
	p += 3;
      }
    }
    return nnz;
  }
};


template <typename T>
Array FM3Sparse(const QVector<QVector<T> >& arg, NTuple dims) {
  SparseMatrix<T> retval(dims);
  for (int i=0;i<arg.size();i++) {
    RLEDecoder<T> rle(arg[i]);
    rle.update();
    while (rle.more()) {
      retval[NTuple(rle.row()+1,i+1)] = rle.value();
      rle.advance();
    }
  }
  return Array(retval);
}

template <typename T>
Array FM3SparseComplex(const QVector<QVector<T> >&arg, NTuple dims) {
  SparseMatrix<T> real;
  SparseMatrix<T> imag;
  for (int i=0;i<arg.size();i++) {
    RLEDecoderComplex<T> rle(arg[i]);
    rle.update();
    while (rle.more()) {
      real[NTuple(rle.row()+1,i+1)] = rle.value_real();
      imag[NTuple(rle.row()+1,i+1)] = rle.value_imag();
      rle.advance();
    }
  }
  return Array(real,imag);
}

template <typename T>
QVector<QVector<T> > SparseFM3(const SparseMatrix<T>& arg) {
  QVector<QVector<T> > ret(int(arg.cols()));
  ConstSparseIterator<T> iter(&arg);
  while (iter.isValid()) {
    RLEEncoder<T> rle;
    while (iter.moreInSlice()) {
      rle.set(int(iter.row()-1));
      rle.push(iter.value());
      iter.next();
    }
    rle.end(int(arg.rows()));
    ret[int(iter.col()-1)] = rle.copyout();
    iter.nextSlice();
  }
  return ret;
}

template <typename T>
QVector<QVector<T> > SparseFM3(const SparseMatrix<T>& real, const SparseMatrix<T>& imag) {
  QVector<QVector<T> > ret(int(real.cols()));
  ConstComplexSparseIterator<T> iter(&real,&imag);
  while (iter.isValid()) {
    RLEEncoderComplex<T> rle;
    while (iter.moreInSlice()) {
      rle.set(int(iter.row()-1));
      rle.push(iter.realValue(),iter.imagValue());
      iter.next();
    }
    rle.end(int(real.rows()));
    ret[int(iter.col()-1)] = rle.copyout();
    iter.nextSlice();
  }
  return ret;
}


#endif
