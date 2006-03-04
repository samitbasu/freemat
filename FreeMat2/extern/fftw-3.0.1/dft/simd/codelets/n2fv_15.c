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
/* Generated on Sat Jul  5 21:40:37 EDT 2003 */

#include "codelet-dft.h"

/* Generated by: /homee/stevenj/cvs/fftw3.0.1/genfft/gen_notw_c -simd -compact -variables 4 -n 15 -name n2fv_15 -with-ostride 2 -include n2f.h */

/*
 * This function contains 78 FP additions, 25 FP multiplications,
 * (or, 64 additions, 11 multiplications, 14 fused multiply/add),
 * 55 stack variables, and 30 memory accesses
 */
/*
 * Generator Id's : 
 * $Id$
 * $Id$
 * $Id$
 */

#include "n2f.h"

static void n2fv_15(const R *ri, const R *ii, R *ro, R *io, stride is, stride os, int v, int ivs, int ovs)
{
     DVK(KP216506350, +0.216506350946109661690930792688234045867850657);
     DVK(KP509036960, +0.509036960455127183450980863393907648510733164);
     DVK(KP823639103, +0.823639103546331925877420039278190003029660514);
     DVK(KP587785252, +0.587785252292473129168705954639072768597652438);
     DVK(KP951056516, +0.951056516295153572116439333379382143405698634);
     DVK(KP250000000, +0.250000000000000000000000000000000000000000000);
     DVK(KP559016994, +0.559016994374947424102293417182819058860154590);
     DVK(KP866025403, +0.866025403784438646763723170752936183471402627);
     DVK(KP484122918, +0.484122918275927110647408174972799951354115213);
     DVK(KP500000000, +0.500000000000000000000000000000000000000000000);
     int i;
     const R *xi;
     R *xo;
     xi = ri;
     xo = ro;
     BEGIN_SIMD();
     for (i = v; i > 0; i = i - VL, xi = xi + (VL * ivs), xo = xo + (VL * ovs)) {
	  V T5, T10, TB, TO, TU, TV, TR, Ta, Tf, Tg, Tl, Tq, Tr, TE, TH;
	  V TI, TZ, T11, T1f, T1g;
	  {
	       V T1, T2, T3, T4;
	       T1 = LD(&(xi[0]), ivs, &(xi[0]));
	       T2 = LD(&(xi[WS(is, 5)]), ivs, &(xi[WS(is, 1)]));
	       T3 = LD(&(xi[WS(is, 10)]), ivs, &(xi[0]));
	       T4 = VADD(T2, T3);
	       T5 = VADD(T1, T4);
	       T10 = VSUB(T3, T2);
	       TB = VFNMS(LDK(KP500000000), T4, T1);
	  }
	  {
	       V T6, T9, TC, TP, Tm, Tp, TG, TN, Tb, Te, TD, TQ, Th, Tk, TF;
	       V TM, TX, TY;
	       {
		    V T7, T8, Tn, To;
		    T6 = LD(&(xi[WS(is, 3)]), ivs, &(xi[WS(is, 1)]));
		    T7 = LD(&(xi[WS(is, 8)]), ivs, &(xi[0]));
		    T8 = LD(&(xi[WS(is, 13)]), ivs, &(xi[WS(is, 1)]));
		    T9 = VADD(T7, T8);
		    TC = VFNMS(LDK(KP500000000), T9, T6);
		    TP = VSUB(T8, T7);
		    Tm = LD(&(xi[WS(is, 9)]), ivs, &(xi[WS(is, 1)]));
		    Tn = LD(&(xi[WS(is, 14)]), ivs, &(xi[0]));
		    To = LD(&(xi[WS(is, 4)]), ivs, &(xi[0]));
		    Tp = VADD(Tn, To);
		    TG = VFNMS(LDK(KP500000000), Tp, Tm);
		    TN = VSUB(To, Tn);
	       }
	       {
		    V Tc, Td, Ti, Tj;
		    Tb = LD(&(xi[WS(is, 12)]), ivs, &(xi[0]));
		    Tc = LD(&(xi[WS(is, 2)]), ivs, &(xi[0]));
		    Td = LD(&(xi[WS(is, 7)]), ivs, &(xi[WS(is, 1)]));
		    Te = VADD(Tc, Td);
		    TD = VFNMS(LDK(KP500000000), Te, Tb);
		    TQ = VSUB(Td, Tc);
		    Th = LD(&(xi[WS(is, 6)]), ivs, &(xi[0]));
		    Ti = LD(&(xi[WS(is, 11)]), ivs, &(xi[WS(is, 1)]));
		    Tj = LD(&(xi[WS(is, 1)]), ivs, &(xi[WS(is, 1)]));
		    Tk = VADD(Ti, Tj);
		    TF = VFNMS(LDK(KP500000000), Tk, Th);
		    TM = VSUB(Tj, Ti);
	       }
	       TO = VSUB(TM, TN);
	       TU = VSUB(TF, TG);
	       TV = VSUB(TC, TD);
	       TR = VSUB(TP, TQ);
	       Ta = VADD(T6, T9);
	       Tf = VADD(Tb, Te);
	       Tg = VADD(Ta, Tf);
	       Tl = VADD(Th, Tk);
	       Tq = VADD(Tm, Tp);
	       Tr = VADD(Tl, Tq);
	       TE = VADD(TC, TD);
	       TH = VADD(TF, TG);
	       TI = VADD(TE, TH);
	       TX = VADD(TP, TQ);
	       TY = VADD(TM, TN);
	       TZ = VMUL(LDK(KP484122918), VSUB(TX, TY));
	       T11 = VADD(TX, TY);
	  }
	  T1f = VADD(TB, TI);
	  T1g = VBYI(VMUL(LDK(KP866025403), VADD(T10, T11)));
	  ST(&(xo[10]), VSUB(T1f, T1g), ovs, &(xo[2]));
	  ST(&(xo[20]), VADD(T1f, T1g), ovs, &(xo[0]));
	  {
	       V Tu, Ts, Tt, Ty, TA, Tw, Tx, Tz, Tv;
	       Tu = VMUL(LDK(KP559016994), VSUB(Tg, Tr));
	       Ts = VADD(Tg, Tr);
	       Tt = VFNMS(LDK(KP250000000), Ts, T5);
	       Tw = VSUB(Tl, Tq);
	       Tx = VSUB(Ta, Tf);
	       Ty = VBYI(VFNMS(LDK(KP587785252), Tx, VMUL(LDK(KP951056516), Tw)));
	       TA = VBYI(VFMA(LDK(KP951056516), Tx, VMUL(LDK(KP587785252), Tw)));
	       ST(&(xo[0]), VADD(T5, Ts), ovs, &(xo[0]));
	       Tz = VADD(Tu, Tt);
	       ST(&(xo[12]), VSUB(Tz, TA), ovs, &(xo[0]));
	       ST(&(xo[18]), VADD(TA, Tz), ovs, &(xo[2]));
	       Tv = VSUB(Tt, Tu);
	       ST(&(xo[6]), VSUB(Tv, Ty), ovs, &(xo[2]));
	       ST(&(xo[24]), VADD(Ty, Tv), ovs, &(xo[0]));
	  }
	  {
	       V TS, TW, T1b, T18, T13, T1a, TL, T17, T12, TJ, TK;
	       TS = VFNMS(LDK(KP509036960), TR, VMUL(LDK(KP823639103), TO));
	       TW = VFNMS(LDK(KP587785252), TV, VMUL(LDK(KP951056516), TU));
	       T1b = VFMA(LDK(KP951056516), TV, VMUL(LDK(KP587785252), TU));
	       T18 = VFMA(LDK(KP823639103), TR, VMUL(LDK(KP509036960), TO));
	       T12 = VFNMS(LDK(KP216506350), T11, VMUL(LDK(KP866025403), T10));
	       T13 = VSUB(TZ, T12);
	       T1a = VADD(TZ, T12);
	       TJ = VFNMS(LDK(KP250000000), TI, TB);
	       TK = VMUL(LDK(KP559016994), VSUB(TE, TH));
	       TL = VSUB(TJ, TK);
	       T17 = VADD(TK, TJ);
	       {
		    V TT, T14, T1d, T1e;
		    TT = VSUB(TL, TS);
		    T14 = VBYI(VSUB(TW, T13));
		    ST(&(xo[16]), VSUB(TT, T14), ovs, &(xo[0]));
		    ST(&(xo[14]), VADD(TT, T14), ovs, &(xo[2]));
		    T1d = VSUB(T17, T18);
		    T1e = VBYI(VADD(T1b, T1a));
		    ST(&(xo[22]), VSUB(T1d, T1e), ovs, &(xo[2]));
		    ST(&(xo[8]), VADD(T1d, T1e), ovs, &(xo[0]));
	       }
	       {
		    V T15, T16, T19, T1c;
		    T15 = VADD(TL, TS);
		    T16 = VBYI(VADD(TW, T13));
		    ST(&(xo[26]), VSUB(T15, T16), ovs, &(xo[2]));
		    ST(&(xo[4]), VADD(T15, T16), ovs, &(xo[0]));
		    T19 = VADD(T17, T18);
		    T1c = VBYI(VSUB(T1a, T1b));
		    ST(&(xo[28]), VSUB(T19, T1c), ovs, &(xo[0]));
		    ST(&(xo[2]), VADD(T19, T1c), ovs, &(xo[2]));
	       }
	  }
     }
     END_SIMD();
}

static const kdft_desc desc = { 15, "n2fv_15", {64, 11, 14, 0}, &GENUS, 0, 2, 0, 0 };
void X(codelet_n2fv_15) (planner *p) {
     X(kdft_register) (p, n2fv_15, &desc);
}
