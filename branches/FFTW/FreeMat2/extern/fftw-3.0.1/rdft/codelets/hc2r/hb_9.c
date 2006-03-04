/*
 * Copyright (c) 2003 Matteo Frigo
 * Copyright (c) 2003 Massachusetts Institute of Technology
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
 *
 */

/* This file was automatically generated --- DO NOT EDIT */
/* Generated on Sat Jul  5 22:11:31 EDT 2003 */

#include "codelet-rdft.h"

/* Generated by: /homee/stevenj/cvs/fftw3.0.1/genfft/gen_hc2hc -compact -variables 4 -sign 1 -n 9 -dif -name hb_9 -include hb.h */

/*
 * This function contains 96 FP additions, 72 FP multiplications,
 * (or, 60 additions, 36 multiplications, 36 fused multiply/add),
 * 53 stack variables, and 36 memory accesses
 */
/*
 * Generator Id's : 
 * $Id$
 * $Id$
 * $Id$
 */

#include "hb.h"

static const R *hb_9(R *rio, R *iio, const R *W, stride ios, int m, int dist)
{
     DK(KP642787609, +0.642787609686539326322643409907263432907559884);
     DK(KP766044443, +0.766044443118978035202392650555416673935832457);
     DK(KP984807753, +0.984807753012208059366743024589523013670643252);
     DK(KP173648177, +0.173648177666930348851716626769314796000375677);
     DK(KP342020143, +0.342020143325668733044099614682259580763083368);
     DK(KP939692620, +0.939692620785908384054109277324731469936208134);
     DK(KP500000000, +0.500000000000000000000000000000000000000000000);
     DK(KP866025403, +0.866025403784438646763723170752936183471402627);
     int i;
     for (i = m - 2; i > 0; i = i - 2, rio = rio + dist, iio = iio - dist, W = W + 16) {
	  E T5, T1z, Tm, T18, TQ, T1i, Ta, Tf, Tg, T1A, T1B, T1C, Tx, TS, T1e;
	  E T1k, T1b, T1j, TI, TR;
	  {
	       E T1, TM, T4, TP, Tl, TN, Ti, TO;
	       T1 = rio[0];
	       TM = iio[0];
	       {
		    E T2, T3, Tj, Tk;
		    T2 = rio[WS(ios, 3)];
		    T3 = iio[-WS(ios, 6)];
		    T4 = T2 + T3;
		    TP = KP866025403 * (T2 - T3);
		    Tj = rio[WS(ios, 6)];
		    Tk = iio[-WS(ios, 3)];
		    Tl = KP866025403 * (Tj + Tk);
		    TN = Tj - Tk;
	       }
	       T5 = T1 + T4;
	       T1z = TM - TN;
	       Ti = FNMS(KP500000000, T4, T1);
	       Tm = Ti + Tl;
	       T18 = Ti - Tl;
	       TO = FMA(KP500000000, TN, TM);
	       TQ = TO - TP;
	       T1i = TP + TO;
	  }
	  {
	       E T6, T9, Ty, TG, TD, TE, TB, TF, Tb, Te, Tn, Tv, Tt, Ts, Tq;
	       E Tu;
	       {
		    E T7, T8, Tz, TA;
		    T6 = rio[WS(ios, 1)];
		    T7 = rio[WS(ios, 4)];
		    T8 = iio[-WS(ios, 7)];
		    T9 = T7 + T8;
		    Ty = FNMS(KP500000000, T9, T6);
		    TG = KP866025403 * (T7 - T8);
		    TD = iio[-WS(ios, 1)];
		    Tz = rio[WS(ios, 7)];
		    TA = iio[-WS(ios, 4)];
		    TE = Tz - TA;
		    TB = KP866025403 * (Tz + TA);
		    TF = FMA(KP500000000, TE, TD);
	       }
	       {
		    E Tc, Td, To, Tp;
		    Tb = rio[WS(ios, 2)];
		    Tc = iio[-WS(ios, 5)];
		    Td = iio[-WS(ios, 8)];
		    Te = Tc + Td;
		    Tn = FNMS(KP500000000, Te, Tb);
		    Tv = KP866025403 * (Tc - Td);
		    Tt = iio[-WS(ios, 2)];
		    To = rio[WS(ios, 5)];
		    Tp = rio[WS(ios, 8)];
		    Ts = To + Tp;
		    Tq = KP866025403 * (To - Tp);
		    Tu = FMA(KP500000000, Ts, Tt);
	       }
	       {
		    E Tr, Tw, T1c, T1d;
		    Ta = T6 + T9;
		    Tf = Tb + Te;
		    Tg = Ta + Tf;
		    T1A = TD - TE;
		    T1B = Tt - Ts;
		    T1C = T1A + T1B;
		    Tr = Tn - Tq;
		    Tw = Tu - Tv;
		    Tx = FMA(KP939692620, Tr, KP342020143 * Tw);
		    TS = FNMS(KP939692620, Tw, KP342020143 * Tr);
		    T1c = Tn + Tq;
		    T1d = Tv + Tu;
		    T1e = FNMS(KP984807753, T1d, KP173648177 * T1c);
		    T1k = FMA(KP984807753, T1c, KP173648177 * T1d);
		    {
			 E T19, T1a, TC, TH;
			 T19 = Ty - TB;
			 T1a = TG + TF;
			 T1b = FNMS(KP642787609, T1a, KP766044443 * T19);
			 T1j = FMA(KP766044443, T1a, KP642787609 * T19);
			 TC = Ty + TB;
			 TH = TF - TG;
			 TI = FNMS(KP984807753, TH, KP173648177 * TC);
			 TR = FMA(KP173648177, TH, KP984807753 * TC);
		    }
	       }
	  }
	  rio[0] = T5 + Tg;
	  {
	       E TX, T11, TK, T10, TU, TW, TJ, TT, Th, TL;
	       TX = KP866025403 * (TI + Tx);
	       T11 = KP866025403 * (TS - TR);
	       TJ = Tx - TI;
	       TK = Tm - TJ;
	       T10 = FMA(KP500000000, TJ, Tm);
	       TT = TR + TS;
	       TU = TQ + TT;
	       TW = FNMS(KP500000000, TT, TQ);
	       Th = W[2];
	       TL = W[3];
	       rio[WS(ios, 2)] = FNMS(TL, TU, Th * TK);
	       iio[-WS(ios, 6)] = FMA(Th, TU, TL * TK);
	       {
		    E T14, T16, T13, T15;
		    T14 = TW + TX;
		    T16 = T11 + T10;
		    T13 = W[8];
		    T15 = W[9];
		    iio[-WS(ios, 3)] = FMA(T13, T14, T15 * T16);
		    rio[WS(ios, 5)] = FNMS(T15, T14, T13 * T16);
	       }
	       {
		    E TY, T12, TV, TZ;
		    TY = TW - TX;
		    T12 = T10 - T11;
		    TV = W[14];
		    TZ = W[15];
		    iio[0] = FMA(TV, TY, TZ * T12);
		    rio[WS(ios, 8)] = FNMS(TZ, TY, TV * T12);
	       }
	  }
	  iio[-WS(ios, 8)] = T1z + T1C;
	  {
	       E T1G, T1O, T1K, T1M;
	       {
		    E T1E, T1F, T1I, T1J;
		    T1E = FNMS(KP500000000, T1C, T1z);
		    T1F = KP866025403 * (Ta - Tf);
		    T1G = T1E - T1F;
		    T1O = T1F + T1E;
		    T1I = FNMS(KP500000000, Tg, T5);
		    T1J = KP866025403 * (T1B - T1A);
		    T1K = T1I - T1J;
		    T1M = T1I + T1J;
	       }
	       {
		    E T1D, T1H, T1L, T1N;
		    T1D = W[10];
		    T1H = W[11];
		    iio[-WS(ios, 2)] = FMA(T1D, T1G, T1H * T1K);
		    rio[WS(ios, 6)] = FNMS(T1H, T1G, T1D * T1K);
		    T1L = W[4];
		    T1N = W[5];
		    rio[WS(ios, 3)] = FNMS(T1N, T1O, T1L * T1M);
		    iio[-WS(ios, 5)] = FMA(T1L, T1O, T1N * T1M);
	       }
	  }
	  {
	       E T1p, T1t, T1g, T1s, T1m, T1o, T1f, T1l, T17, T1h;
	       T1p = KP866025403 * (T1b - T1e);
	       T1t = KP866025403 * (T1k - T1j);
	       T1f = T1b + T1e;
	       T1g = T18 + T1f;
	       T1s = FNMS(KP500000000, T1f, T18);
	       T1l = T1j + T1k;
	       T1m = T1i + T1l;
	       T1o = FNMS(KP500000000, T1l, T1i);
	       T17 = W[0];
	       T1h = W[1];
	       rio[WS(ios, 1)] = FNMS(T1h, T1m, T17 * T1g);
	       iio[-WS(ios, 7)] = FMA(T1h, T1g, T17 * T1m);
	       {
		    E T1q, T1u, T1n, T1r;
		    T1q = T1o - T1p;
		    T1u = T1s - T1t;
		    T1n = W[12];
		    T1r = W[13];
		    iio[-WS(ios, 1)] = FMA(T1n, T1q, T1r * T1u);
		    rio[WS(ios, 7)] = FNMS(T1r, T1q, T1n * T1u);
	       }
	       {
		    E T1w, T1y, T1v, T1x;
		    T1w = T1s + T1t;
		    T1y = T1p + T1o;
		    T1v = W[6];
		    T1x = W[7];
		    rio[WS(ios, 4)] = FNMS(T1x, T1y, T1v * T1w);
		    iio[-WS(ios, 4)] = FMA(T1v, T1y, T1x * T1w);
	       }
	  }
     }
     return W;
}

static const tw_instr twinstr[] = {
     {TW_FULL, 0, 9},
     {TW_NEXT, 1, 0}
};

static const hc2hc_desc desc = { 9, "hb_9", twinstr, {60, 36, 36, 0}, &GENUS, 0, 0, 0 };

void X(codelet_hb_9) (planner *p) {
     X(khc2hc_dif_register) (p, hb_9, &desc);
}
