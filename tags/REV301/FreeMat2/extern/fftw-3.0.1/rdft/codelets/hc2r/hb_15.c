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
/* Generated on Sat Jul  5 22:11:36 EDT 2003 */

#include "codelet-rdft.h"

/* Generated by: /homee/stevenj/cvs/fftw3.0.1/genfft/gen_hc2hc -compact -variables 4 -sign 1 -n 15 -dif -name hb_15 -include hb.h */

/*
 * This function contains 184 FP additions, 112 FP multiplications,
 * (or, 128 additions, 56 multiplications, 56 fused multiply/add),
 * 75 stack variables, and 60 memory accesses
 */
/*
 * Generator Id's : 
 * $Id$
 * $Id$
 * $Id$
 */

#include "hb.h"

static const R *hb_15(R *rio, R *iio, const R *W, stride ios, int m, int dist)
{
     DK(KP250000000, +0.250000000000000000000000000000000000000000000);
     DK(KP559016994, +0.559016994374947424102293417182819058860154590);
     DK(KP587785252, +0.587785252292473129168705954639072768597652438);
     DK(KP951056516, +0.951056516295153572116439333379382143405698634);
     DK(KP500000000, +0.500000000000000000000000000000000000000000000);
     DK(KP866025403, +0.866025403784438646763723170752936183471402627);
     int i;
     for (i = m - 2; i > 0; i = i - 2, rio = rio + dist, iio = iio - dist, W = W + 28) {
	  E T5, T2N, TV, T25, T1v, T2o, T2W, T38, T37, T2X, T2Q, T2T, T2U, Tg, Tr;
	  E Ts, Ty, TD, TE, T2c, T2d, T2m, TJ, TO, TP, T1Y, T1Z, T20, T1e, T1j;
	  E T1p, T13, T18, T1o, T29, T2a, T2l, T21, T22, T23;
	  {
	       E T1, T1r, T4, T1u, TU, T1s, TR, T1t;
	       T1 = rio[0];
	       T1r = iio[0];
	       {
		    E T2, T3, TS, TT;
		    T2 = rio[WS(ios, 5)];
		    T3 = iio[-WS(ios, 10)];
		    T4 = T2 + T3;
		    T1u = KP866025403 * (T2 - T3);
		    TS = rio[WS(ios, 10)];
		    TT = iio[-WS(ios, 5)];
		    TU = KP866025403 * (TS + TT);
		    T1s = TS - TT;
	       }
	       T5 = T1 + T4;
	       T2N = T1r - T1s;
	       TR = FNMS(KP500000000, T4, T1);
	       TV = TR + TU;
	       T25 = TR - TU;
	       T1t = FMA(KP500000000, T1s, T1r);
	       T1v = T1t - T1u;
	       T2o = T1u + T1t;
	  }
	  {
	       E Ta, T12, Tu, Tx, T2O, T11, Tf, T14, Tz, TC, T2P, T17, Tl, T1d, TF;
	       E TI, T2R, T1c, Tq, T1i, TK, TN, T2S, T1h;
	       {
		    E T6, T7, T8, T9;
		    T6 = rio[WS(ios, 3)];
		    T7 = iio[-WS(ios, 8)];
		    T8 = iio[-WS(ios, 13)];
		    T9 = T7 + T8;
		    Ta = T6 + T9;
		    T12 = KP866025403 * (T7 - T8);
		    Tu = FNMS(KP500000000, T9, T6);
	       }
	       {
		    E TZ, Tv, Tw, T10;
		    TZ = iio[-WS(ios, 3)];
		    Tv = rio[WS(ios, 8)];
		    Tw = rio[WS(ios, 13)];
		    T10 = Tv + Tw;
		    Tx = KP866025403 * (Tv - Tw);
		    T2O = TZ - T10;
		    T11 = FMA(KP500000000, T10, TZ);
	       }
	       {
		    E Tb, Tc, Td, Te;
		    Tb = iio[-WS(ios, 12)];
		    Tc = rio[WS(ios, 2)];
		    Td = rio[WS(ios, 7)];
		    Te = Tc + Td;
		    Tf = Tb + Te;
		    T14 = KP866025403 * (Tc - Td);
		    Tz = FNMS(KP500000000, Te, Tb);
	       }
	       {
		    E T15, TA, TB, T16;
		    T15 = rio[WS(ios, 12)];
		    TA = iio[-WS(ios, 7)];
		    TB = iio[-WS(ios, 2)];
		    T16 = TB + TA;
		    TC = KP866025403 * (TA - TB);
		    T2P = T16 - T15;
		    T17 = FMA(KP500000000, T16, T15);
	       }
	       {
		    E Th, Ti, Tj, Tk;
		    Th = rio[WS(ios, 6)];
		    Ti = iio[-WS(ios, 11)];
		    Tj = rio[WS(ios, 1)];
		    Tk = Ti + Tj;
		    Tl = Th + Tk;
		    T1d = KP866025403 * (Ti - Tj);
		    TF = FNMS(KP500000000, Tk, Th);
	       }
	       {
		    E T1a, TG, TH, T1b;
		    T1a = iio[-WS(ios, 6)];
		    TG = rio[WS(ios, 11)];
		    TH = iio[-WS(ios, 1)];
		    T1b = TG - TH;
		    TI = KP866025403 * (TG + TH);
		    T2R = T1a - T1b;
		    T1c = FMA(KP500000000, T1b, T1a);
	       }
	       {
		    E Tm, Tn, To, Tp;
		    Tm = iio[-WS(ios, 9)];
		    Tn = iio[-WS(ios, 14)];
		    To = rio[WS(ios, 4)];
		    Tp = Tn + To;
		    Tq = Tm + Tp;
		    T1i = KP866025403 * (Tn - To);
		    TK = FNMS(KP500000000, Tp, Tm);
	       }
	       {
		    E T1g, TL, TM, T1f;
		    T1g = rio[WS(ios, 9)];
		    TL = rio[WS(ios, 14)];
		    TM = iio[-WS(ios, 4)];
		    T1f = TL - TM;
		    TN = KP866025403 * (TL + TM);
		    T2S = T1f + T1g;
		    T1h = FMS(KP500000000, T1f, T1g);
	       }
	       T2W = Ta - Tf;
	       T38 = T2R + T2S;
	       T37 = T2O - T2P;
	       T2X = Tl - Tq;
	       T2Q = T2O + T2P;
	       T2T = T2R - T2S;
	       T2U = T2Q + T2T;
	       Tg = Ta + Tf;
	       Tr = Tl + Tq;
	       Ts = Tg + Tr;
	       Ty = Tu - Tx;
	       TD = Tz - TC;
	       TE = Ty + TD;
	       T2c = T1d + T1c;
	       T2d = T1i + T1h;
	       T2m = T2c + T2d;
	       TJ = TF - TI;
	       TO = TK - TN;
	       TP = TJ + TO;
	       T1Y = Tu + Tx;
	       T1Z = Tz + TC;
	       T20 = T1Y + T1Z;
	       T1e = T1c - T1d;
	       T1j = T1h - T1i;
	       T1p = T1e + T1j;
	       T13 = T11 - T12;
	       T18 = T14 + T17;
	       T1o = T13 - T18;
	       T29 = T12 + T11;
	       T2a = T14 - T17;
	       T2l = T29 + T2a;
	       T21 = TF + TI;
	       T22 = TK + TN;
	       T23 = T21 + T22;
	  }
	  rio[0] = T5 + Ts;
	  {
	       E T1l, T1J, T1B, T1M, TY, T1U, T1I, T1y, T1W, T1N, T1T, T1V;
	       {
		    E T19, T1k, T1z, T1A;
		    T19 = T13 + T18;
		    T1k = T1e - T1j;
		    T1l = FMA(KP951056516, T19, KP587785252 * T1k);
		    T1J = FNMS(KP951056516, T1k, KP587785252 * T19);
		    T1z = Ty - TD;
		    T1A = TJ - TO;
		    T1B = FMA(KP951056516, T1z, KP587785252 * T1A);
		    T1M = FNMS(KP951056516, T1A, KP587785252 * T1z);
	       }
	       {
		    E TQ, TW, TX, T1q, T1w, T1x;
		    TQ = KP559016994 * (TE - TP);
		    TW = TE + TP;
		    TX = FNMS(KP250000000, TW, TV);
		    TY = TQ + TX;
		    T1U = TV + TW;
		    T1I = TX - TQ;
		    T1q = KP559016994 * (T1o - T1p);
		    T1w = T1o + T1p;
		    T1x = FNMS(KP250000000, T1w, T1v);
		    T1y = T1q + T1x;
		    T1W = T1v + T1w;
		    T1N = T1x - T1q;
	       }
	       T1T = W[8];
	       T1V = W[9];
	       rio[WS(ios, 5)] = FNMS(T1V, T1W, T1T * T1U);
	       iio[-WS(ios, 9)] = FMA(T1V, T1U, T1T * T1W);
	       {
		    E T1Q, T1S, T1P, T1R;
		    T1Q = T1I + T1J;
		    T1S = T1N - T1M;
		    T1P = W[14];
		    T1R = W[15];
		    rio[WS(ios, 8)] = FNMS(T1R, T1S, T1P * T1Q);
		    iio[-WS(ios, 6)] = FMA(T1R, T1Q, T1P * T1S);
	       }
	       {
		    E T1m, T1C, Tt, T1n;
		    T1m = TY + T1l;
		    T1C = T1y - T1B;
		    Tt = W[26];
		    T1n = W[27];
		    rio[WS(ios, 14)] = FNMS(T1n, T1C, Tt * T1m);
		    iio[0] = FMA(T1n, T1m, Tt * T1C);
	       }
	       {
		    E T1E, T1G, T1D, T1F;
		    T1E = TY - T1l;
		    T1G = T1B + T1y;
		    T1D = W[20];
		    T1F = W[21];
		    rio[WS(ios, 11)] = FNMS(T1F, T1G, T1D * T1E);
		    iio[-WS(ios, 3)] = FMA(T1F, T1E, T1D * T1G);
	       }
	       {
		    E T1K, T1O, T1H, T1L;
		    T1K = T1I - T1J;
		    T1O = T1M + T1N;
		    T1H = W[2];
		    T1L = W[3];
		    rio[WS(ios, 2)] = FNMS(T1L, T1O, T1H * T1K);
		    iio[-WS(ios, 12)] = FMA(T1L, T1K, T1H * T1O);
	       }
	  }
	  iio[-WS(ios, 14)] = T2N + T2U;
	  {
	       E T2Y, T39, T3k, T3h, T36, T3g, T31, T3l;
	       T2Y = FNMS(KP951056516, T2X, KP587785252 * T2W);
	       T39 = FNMS(KP951056516, T38, KP587785252 * T37);
	       T3k = FMA(KP951056516, T2W, KP587785252 * T2X);
	       T3h = FMA(KP951056516, T37, KP587785252 * T38);
	       {
		    E T34, T35, T2Z, T30;
		    T34 = FNMS(KP250000000, Ts, T5);
		    T35 = KP559016994 * (Tg - Tr);
		    T36 = T34 - T35;
		    T3g = T35 + T34;
		    T2Z = FNMS(KP250000000, T2U, T2N);
		    T30 = KP559016994 * (T2Q - T2T);
		    T31 = T2Z - T30;
		    T3l = T30 + T2Z;
	       }
	       {
		    E T32, T3a, T2V, T33;
		    T32 = T2Y + T31;
		    T3a = T36 - T39;
		    T2V = W[22];
		    T33 = W[23];
		    iio[-WS(ios, 2)] = FMA(T2V, T32, T33 * T3a);
		    rio[WS(ios, 12)] = FNMS(T33, T32, T2V * T3a);
	       }
	       {
		    E T3o, T3q, T3n, T3p;
		    T3o = T3l - T3k;
		    T3q = T3g + T3h;
		    T3n = W[16];
		    T3p = W[17];
		    iio[-WS(ios, 5)] = FMA(T3n, T3o, T3p * T3q);
		    rio[WS(ios, 9)] = FNMS(T3p, T3o, T3n * T3q);
	       }
	       {
		    E T3c, T3e, T3b, T3d;
		    T3c = T36 + T39;
		    T3e = T31 - T2Y;
		    T3b = W[4];
		    T3d = W[5];
		    rio[WS(ios, 3)] = FNMS(T3d, T3e, T3b * T3c);
		    iio[-WS(ios, 11)] = FMA(T3b, T3e, T3d * T3c);
	       }
	       {
		    E T3i, T3m, T3f, T3j;
		    T3i = T3g - T3h;
		    T3m = T3k + T3l;
		    T3f = W[10];
		    T3j = W[11];
		    rio[WS(ios, 6)] = FNMS(T3j, T3m, T3f * T3i);
		    iio[-WS(ios, 8)] = FMA(T3f, T3m, T3j * T3i);
	       }
	  }
	  {
	       E T2f, T2z, T2k, T2D, T28, T2K, T2y, T2r, T2M, T2C, T2J, T2L;
	       {
		    E T2b, T2e, T2i, T2j;
		    T2b = T29 - T2a;
		    T2e = T2c - T2d;
		    T2f = FMA(KP951056516, T2b, KP587785252 * T2e);
		    T2z = FNMS(KP951056516, T2e, KP587785252 * T2b);
		    T2i = T1Y - T1Z;
		    T2j = T21 - T22;
		    T2k = FMA(KP951056516, T2i, KP587785252 * T2j);
		    T2D = FNMS(KP951056516, T2j, KP587785252 * T2i);
	       }
	       {
		    E T24, T26, T27, T2n, T2p, T2q;
		    T24 = KP559016994 * (T20 - T23);
		    T26 = T20 + T23;
		    T27 = FNMS(KP250000000, T26, T25);
		    T28 = T24 + T27;
		    T2K = T25 + T26;
		    T2y = T27 - T24;
		    T2n = KP559016994 * (T2l - T2m);
		    T2p = T2l + T2m;
		    T2q = FNMS(KP250000000, T2p, T2o);
		    T2r = T2n + T2q;
		    T2M = T2o + T2p;
		    T2C = T2q - T2n;
	       }
	       T2J = W[18];
	       T2L = W[19];
	       rio[WS(ios, 10)] = FNMS(T2L, T2M, T2J * T2K);
	       iio[-WS(ios, 4)] = FMA(T2L, T2K, T2J * T2M);
	       {
		    E T2u, T2w, T2t, T2v;
		    T2u = T28 + T2f;
		    T2w = T2r - T2k;
		    T2t = W[6];
		    T2v = W[7];
		    rio[WS(ios, 4)] = FNMS(T2v, T2w, T2t * T2u);
		    iio[-WS(ios, 10)] = FMA(T2v, T2u, T2t * T2w);
	       }
	       {
		    E T2g, T2s, T1X, T2h;
		    T2g = T28 - T2f;
		    T2s = T2k + T2r;
		    T1X = W[0];
		    T2h = W[1];
		    rio[WS(ios, 1)] = FNMS(T2h, T2s, T1X * T2g);
		    iio[-WS(ios, 13)] = FMA(T2h, T2g, T1X * T2s);
	       }
	       {
		    E T2A, T2E, T2x, T2B;
		    T2A = T2y + T2z;
		    T2E = T2C - T2D;
		    T2x = W[24];
		    T2B = W[25];
		    rio[WS(ios, 13)] = FNMS(T2B, T2E, T2x * T2A);
		    iio[-WS(ios, 1)] = FMA(T2B, T2A, T2x * T2E);
	       }
	       {
		    E T2G, T2I, T2F, T2H;
		    T2G = T2y - T2z;
		    T2I = T2D + T2C;
		    T2F = W[12];
		    T2H = W[13];
		    rio[WS(ios, 7)] = FNMS(T2H, T2I, T2F * T2G);
		    iio[-WS(ios, 7)] = FMA(T2H, T2G, T2F * T2I);
	       }
	  }
     }
     return W;
}

static const tw_instr twinstr[] = {
     {TW_FULL, 0, 15},
     {TW_NEXT, 1, 0}
};

static const hc2hc_desc desc = { 15, "hb_15", twinstr, {128, 56, 56, 0}, &GENUS, 0, 0, 0 };

void X(codelet_hb_15) (planner *p) {
     X(khc2hc_dif_register) (p, hb_15, &desc);
}
