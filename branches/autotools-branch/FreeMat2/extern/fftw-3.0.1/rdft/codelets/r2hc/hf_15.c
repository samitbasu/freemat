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
/* Generated on Sat Jul  5 21:57:07 EDT 2003 */

#include "codelet-rdft.h"

/* Generated by: /homee/stevenj/cvs/fftw3.0.1/genfft/gen_hc2hc -compact -variables 4 -n 15 -dit -name hf_15 -include hf.h */

/*
 * This function contains 184 FP additions, 112 FP multiplications,
 * (or, 128 additions, 56 multiplications, 56 fused multiply/add),
 * 65 stack variables, and 60 memory accesses
 */
/*
 * Generator Id's : 
 * $Id$
 * $Id$
 * $Id$
 */

#include "hf.h"

static const R *hf_15(R *rio, R *iio, const R *W, stride ios, int m, int dist)
{
     DK(KP587785252, +0.587785252292473129168705954639072768597652438);
     DK(KP951056516, +0.951056516295153572116439333379382143405698634);
     DK(KP250000000, +0.250000000000000000000000000000000000000000000);
     DK(KP559016994, +0.559016994374947424102293417182819058860154590);
     DK(KP500000000, +0.500000000000000000000000000000000000000000000);
     DK(KP866025403, +0.866025403784438646763723170752936183471402627);
     int i;
     for (i = m - 2; i > 0; i = i - 2, rio = rio + dist, iio = iio - dist, W = W + 28) {
	  E T1q, T34, Td, T1n, T2S, T35, T13, T1k, T1l, T2E, T2F, T2O, T1H, T1T, T2k;
	  E T2t, T2f, T2s, T1M, T1U, Tu, TL, TM, T2H, T2I, T2N, T1w, T1Q, T29, T2w;
	  E T24, T2v, T1B, T1R;
	  {
	       E T1, T2R, T6, T1o, Tb, T1p, Tc, T2Q;
	       T1 = rio[0];
	       T2R = iio[-WS(ios, 14)];
	       {
		    E T3, T5, T2, T4;
		    T3 = rio[WS(ios, 5)];
		    T5 = iio[-WS(ios, 9)];
		    T2 = W[8];
		    T4 = W[9];
		    T6 = FMA(T2, T3, T4 * T5);
		    T1o = FNMS(T4, T3, T2 * T5);
	       }
	       {
		    E T8, Ta, T7, T9;
		    T8 = rio[WS(ios, 10)];
		    Ta = iio[-WS(ios, 4)];
		    T7 = W[18];
		    T9 = W[19];
		    Tb = FMA(T7, T8, T9 * Ta);
		    T1p = FNMS(T9, T8, T7 * Ta);
	       }
	       T1q = KP866025403 * (T1o - T1p);
	       T34 = KP866025403 * (Tb - T6);
	       Tc = T6 + Tb;
	       Td = T1 + Tc;
	       T1n = FNMS(KP500000000, Tc, T1);
	       T2Q = T1o + T1p;
	       T2S = T2Q + T2R;
	       T35 = FNMS(KP500000000, T2Q, T2R);
	  }
	  {
	       E TR, T2c, T18, T2h, TW, T1E, T11, T1F, T12, T2d, T1d, T1J, T1i, T1K, T1j;
	       E T2i;
	       {
		    E TO, TQ, TN, TP;
		    TO = rio[WS(ios, 6)];
		    TQ = iio[-WS(ios, 8)];
		    TN = W[10];
		    TP = W[11];
		    TR = FMA(TN, TO, TP * TQ);
		    T2c = FNMS(TP, TO, TN * TQ);
	       }
	       {
		    E T15, T17, T14, T16;
		    T15 = rio[WS(ios, 9)];
		    T17 = iio[-WS(ios, 5)];
		    T14 = W[16];
		    T16 = W[17];
		    T18 = FMA(T14, T15, T16 * T17);
		    T2h = FNMS(T16, T15, T14 * T17);
	       }
	       {
		    E TT, TV, TS, TU;
		    TT = rio[WS(ios, 11)];
		    TV = iio[-WS(ios, 3)];
		    TS = W[20];
		    TU = W[21];
		    TW = FMA(TS, TT, TU * TV);
		    T1E = FNMS(TU, TT, TS * TV);
	       }
	       {
		    E TY, T10, TX, TZ;
		    TY = rio[WS(ios, 1)];
		    T10 = iio[-WS(ios, 13)];
		    TX = W[0];
		    TZ = W[1];
		    T11 = FMA(TX, TY, TZ * T10);
		    T1F = FNMS(TZ, TY, TX * T10);
	       }
	       T12 = TW + T11;
	       T2d = T1E + T1F;
	       {
		    E T1a, T1c, T19, T1b;
		    T1a = rio[WS(ios, 14)];
		    T1c = iio[0];
		    T19 = W[26];
		    T1b = W[27];
		    T1d = FMA(T19, T1a, T1b * T1c);
		    T1J = FNMS(T1b, T1a, T19 * T1c);
	       }
	       {
		    E T1f, T1h, T1e, T1g;
		    T1f = rio[WS(ios, 4)];
		    T1h = iio[-WS(ios, 10)];
		    T1e = W[6];
		    T1g = W[7];
		    T1i = FMA(T1e, T1f, T1g * T1h);
		    T1K = FNMS(T1g, T1f, T1e * T1h);
	       }
	       T1j = T1d + T1i;
	       T2i = T1J + T1K;
	       {
		    E T1D, T1G, T2g, T2j;
		    T13 = TR + T12;
		    T1k = T18 + T1j;
		    T1l = T13 + T1k;
		    T2E = T2c + T2d;
		    T2F = T2h + T2i;
		    T2O = T2E + T2F;
		    T1D = FNMS(KP500000000, T12, TR);
		    T1G = KP866025403 * (T1E - T1F);
		    T1H = T1D - T1G;
		    T1T = T1D + T1G;
		    T2g = KP866025403 * (T1d - T1i);
		    T2j = FNMS(KP500000000, T2i, T2h);
		    T2k = T2g - T2j;
		    T2t = T2g + T2j;
		    {
			 E T2b, T2e, T1I, T1L;
			 T2b = KP866025403 * (T11 - TW);
			 T2e = FNMS(KP500000000, T2d, T2c);
			 T2f = T2b + T2e;
			 T2s = T2e - T2b;
			 T1I = FNMS(KP500000000, T1j, T18);
			 T1L = KP866025403 * (T1J - T1K);
			 T1M = T1I - T1L;
			 T1U = T1I + T1L;
		    }
	       }
	  }
	  {
	       E Ti, T21, Tz, T26, TE, T1y, TJ, T1z, TK, T27, Tn, T1t, Ts, T1u, Tt;
	       E T22;
	       {
		    E Tf, Th, Te, Tg;
		    Tf = rio[WS(ios, 3)];
		    Th = iio[-WS(ios, 11)];
		    Te = W[4];
		    Tg = W[5];
		    Ti = FMA(Te, Tf, Tg * Th);
		    T21 = FNMS(Tg, Tf, Te * Th);
	       }
	       {
		    E Tw, Ty, Tv, Tx;
		    Tw = rio[WS(ios, 12)];
		    Ty = iio[-WS(ios, 2)];
		    Tv = W[22];
		    Tx = W[23];
		    Tz = FMA(Tv, Tw, Tx * Ty);
		    T26 = FNMS(Tx, Tw, Tv * Ty);
	       }
	       {
		    E TB, TD, TA, TC;
		    TB = rio[WS(ios, 2)];
		    TD = iio[-WS(ios, 12)];
		    TA = W[2];
		    TC = W[3];
		    TE = FMA(TA, TB, TC * TD);
		    T1y = FNMS(TC, TB, TA * TD);
	       }
	       {
		    E TG, TI, TF, TH;
		    TG = rio[WS(ios, 7)];
		    TI = iio[-WS(ios, 7)];
		    TF = W[12];
		    TH = W[13];
		    TJ = FMA(TF, TG, TH * TI);
		    T1z = FNMS(TH, TG, TF * TI);
	       }
	       TK = TE + TJ;
	       T27 = T1y + T1z;
	       {
		    E Tk, Tm, Tj, Tl;
		    Tk = rio[WS(ios, 8)];
		    Tm = iio[-WS(ios, 6)];
		    Tj = W[14];
		    Tl = W[15];
		    Tn = FMA(Tj, Tk, Tl * Tm);
		    T1t = FNMS(Tl, Tk, Tj * Tm);
	       }
	       {
		    E Tp, Tr, To, Tq;
		    Tp = rio[WS(ios, 13)];
		    Tr = iio[-WS(ios, 1)];
		    To = W[24];
		    Tq = W[25];
		    Ts = FMA(To, Tp, Tq * Tr);
		    T1u = FNMS(Tq, Tp, To * Tr);
	       }
	       Tt = Tn + Ts;
	       T22 = T1t + T1u;
	       {
		    E T1s, T1v, T25, T28;
		    Tu = Ti + Tt;
		    TL = Tz + TK;
		    TM = Tu + TL;
		    T2H = T21 + T22;
		    T2I = T26 + T27;
		    T2N = T2H + T2I;
		    T1s = FNMS(KP500000000, Tt, Ti);
		    T1v = KP866025403 * (T1t - T1u);
		    T1w = T1s - T1v;
		    T1Q = T1s + T1v;
		    T25 = KP866025403 * (TJ - TE);
		    T28 = FNMS(KP500000000, T27, T26);
		    T29 = T25 + T28;
		    T2w = T28 - T25;
		    {
			 E T20, T23, T1x, T1A;
			 T20 = KP866025403 * (Ts - Tn);
			 T23 = FNMS(KP500000000, T22, T21);
			 T24 = T20 + T23;
			 T2v = T23 - T20;
			 T1x = FNMS(KP500000000, TK, Tz);
			 T1A = KP866025403 * (T1y - T1z);
			 T1B = T1x - T1A;
			 T1R = T1x + T1A;
		    }
	       }
	  }
	  {
	       E T2C, T1m, T2B, T2K, T2M, T2G, T2J, T2L, T2D;
	       T2C = KP559016994 * (TM - T1l);
	       T1m = TM + T1l;
	       T2B = FNMS(KP250000000, T1m, Td);
	       T2G = T2E - T2F;
	       T2J = T2H - T2I;
	       T2K = FNMS(KP587785252, T2J, KP951056516 * T2G);
	       T2M = FMA(KP951056516, T2J, KP587785252 * T2G);
	       rio[0] = Td + T1m;
	       T2L = T2C + T2B;
	       iio[-WS(ios, 9)] = T2L - T2M;
	       rio[WS(ios, 6)] = T2L + T2M;
	       T2D = T2B - T2C;
	       iio[-WS(ios, 12)] = T2D - T2K;
	       rio[WS(ios, 3)] = T2D + T2K;
	  }
	  {
	       E T2X, T2P, T2W, T2V, T2Z, T2T, T2U, T30, T2Y;
	       T2X = KP559016994 * (T2N - T2O);
	       T2P = T2N + T2O;
	       T2W = FNMS(KP250000000, T2P, T2S);
	       T2T = Tu - TL;
	       T2U = T1k - T13;
	       T2V = FMA(KP587785252, T2T, KP951056516 * T2U);
	       T2Z = FNMS(KP951056516, T2T, KP587785252 * T2U);
	       iio[0] = T2P + T2S;
	       T30 = T2X + T2W;
	       rio[WS(ios, 9)] = T2Z - T30;
	       iio[-WS(ios, 6)] = T2Z + T30;
	       T2Y = T2W - T2X;
	       rio[WS(ios, 12)] = T2V - T2Y;
	       iio[-WS(ios, 3)] = T2V + T2Y;
	  }
	  {
	       E T2y, T2A, T1r, T1O, T2p, T2q, T2z, T2r;
	       {
		    E T2u, T2x, T1C, T1N;
		    T2u = T2s - T2t;
		    T2x = T2v - T2w;
		    T2y = FNMS(KP587785252, T2x, KP951056516 * T2u);
		    T2A = FMA(KP951056516, T2x, KP587785252 * T2u);
		    T1r = T1n - T1q;
		    T1C = T1w + T1B;
		    T1N = T1H + T1M;
		    T1O = T1C + T1N;
		    T2p = FNMS(KP250000000, T1O, T1r);
		    T2q = KP559016994 * (T1C - T1N);
	       }
	       rio[WS(ios, 5)] = T1r + T1O;
	       T2z = T2q + T2p;
	       iio[-WS(ios, 14)] = T2z - T2A;
	       iio[-WS(ios, 11)] = T2z + T2A;
	       T2r = T2p - T2q;
	       rio[WS(ios, 2)] = T2r - T2y;
	       iio[-WS(ios, 8)] = T2r + T2y;
	  }
	  {
	       E T3h, T3p, T3l, T3m, T3k, T3n, T3q, T3o;
	       {
		    E T3f, T3g, T3i, T3j;
		    T3f = T1w - T1B;
		    T3g = T1H - T1M;
		    T3h = FMA(KP951056516, T3f, KP587785252 * T3g);
		    T3p = FNMS(KP587785252, T3f, KP951056516 * T3g);
		    T3l = T35 - T34;
		    T3i = T2s + T2t;
		    T3j = T2v + T2w;
		    T3m = T3j + T3i;
		    T3k = KP559016994 * (T3i - T3j);
		    T3n = FNMS(KP250000000, T3m, T3l);
	       }
	       iio[-WS(ios, 5)] = T3m + T3l;
	       T3q = T3k + T3n;
	       rio[WS(ios, 8)] = T3p - T3q;
	       iio[-WS(ios, 2)] = T3p + T3q;
	       T3o = T3k - T3n;
	       rio[WS(ios, 11)] = T3h + T3o;
	       rio[WS(ios, 14)] = T3o - T3h;
	  }
	  {
	       E T3c, T3d, T36, T33, T37, T38, T3e, T39;
	       {
		    E T3a, T3b, T31, T32;
		    T3a = T1Q - T1R;
		    T3b = T1T - T1U;
		    T3c = FMA(KP951056516, T3a, KP587785252 * T3b);
		    T3d = FNMS(KP587785252, T3a, KP951056516 * T3b);
		    T36 = T34 + T35;
		    T31 = T2k - T2f;
		    T32 = T24 + T29;
		    T33 = T31 - T32;
		    T37 = KP559016994 * (T32 + T31);
		    T38 = FMA(KP250000000, T33, T36);
	       }
	       rio[WS(ios, 10)] = T33 - T36;
	       T3e = T38 - T37;
	       rio[WS(ios, 13)] = T3d - T3e;
	       iio[-WS(ios, 7)] = T3d + T3e;
	       T39 = T37 + T38;
	       iio[-WS(ios, 1)] = T39 - T3c;
	       iio[-WS(ios, 4)] = T3c + T39;
	  }
	  {
	       E T2m, T2o, T1P, T1W, T1X, T1Y, T2n, T1Z;
	       {
		    E T2a, T2l, T1S, T1V;
		    T2a = T24 - T29;
		    T2l = T2f + T2k;
		    T2m = FMA(KP951056516, T2a, KP587785252 * T2l);
		    T2o = FNMS(KP587785252, T2a, KP951056516 * T2l);
		    T1P = T1n + T1q;
		    T1S = T1Q + T1R;
		    T1V = T1T + T1U;
		    T1W = T1S + T1V;
		    T1X = KP559016994 * (T1S - T1V);
		    T1Y = FNMS(KP250000000, T1W, T1P);
	       }
	       iio[-WS(ios, 10)] = T1P + T1W;
	       T2n = T1Y - T1X;
	       rio[WS(ios, 7)] = T2n - T2o;
	       iio[-WS(ios, 13)] = T2n + T2o;
	       T1Z = T1X + T1Y;
	       rio[WS(ios, 4)] = T1Z - T2m;
	       rio[WS(ios, 1)] = T1Z + T2m;
	  }
     }
     return W;
}

static const tw_instr twinstr[] = {
     {TW_FULL, 0, 15},
     {TW_NEXT, 1, 0}
};

static const hc2hc_desc desc = { 15, "hf_15", twinstr, {128, 56, 56, 0}, &GENUS, 0, 0, 0 };

void X(codelet_hf_15) (planner *p) {
     X(khc2hc_dit_register) (p, hf_15, &desc);
}
