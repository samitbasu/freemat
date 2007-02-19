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
/* Generated on Sat Jul  5 21:45:21 EDT 2003 */

#include "codelet-dft.h"

/* Generated by: /homee/stevenj/cvs/fftw3.0.1/genfft/gen_twidsq_c -simd -compact -variables 4 -n 4 -dif -name q1bv_4 -include q1b.h -sign 1 */

/*
 * This function contains 44 FP additions, 24 FP multiplications,
 * (or, 44 additions, 24 multiplications, 0 fused multiply/add),
 * 22 stack variables, and 32 memory accesses
 */
/*
 * Generator Id's : 
 * $Id$
 * $Id$
 * $Id$
 */

#include "q1b.h"

static const R *q1bv_4(R *ri, R *ii, const R *W, stride is, stride vs, int m, int dist)
{
     int i;
     R *x;
     x = ii;
     BEGIN_SIMD();
     for (i = 0; i < m; i = i + VL, x = x + (VL * dist), W = W + (TWVL * 6)) {
	  V T3, T9, TA, TG, TD, TH, T6, Ta, Te, Tk, Tp, Tv, Ts, Tw, Th;
	  V Tl;
	  {
	       V T1, T2, Ty, Tz;
	       T1 = LD(&(x[0]), dist, &(x[0]));
	       T2 = LD(&(x[WS(is, 2)]), dist, &(x[0]));
	       T3 = VSUB(T1, T2);
	       T9 = VADD(T1, T2);
	       Ty = LD(&(x[WS(vs, 3)]), dist, &(x[WS(vs, 3)]));
	       Tz = LD(&(x[WS(vs, 3) + WS(is, 2)]), dist, &(x[WS(vs, 3)]));
	       TA = VSUB(Ty, Tz);
	       TG = VADD(Ty, Tz);
	  }
	  {
	       V TB, TC, T4, T5;
	       TB = LD(&(x[WS(vs, 3) + WS(is, 1)]), dist, &(x[WS(vs, 3) + WS(is, 1)]));
	       TC = LD(&(x[WS(vs, 3) + WS(is, 3)]), dist, &(x[WS(vs, 3) + WS(is, 1)]));
	       TD = VBYI(VSUB(TB, TC));
	       TH = VADD(TB, TC);
	       T4 = LD(&(x[WS(is, 1)]), dist, &(x[WS(is, 1)]));
	       T5 = LD(&(x[WS(is, 3)]), dist, &(x[WS(is, 1)]));
	       T6 = VBYI(VSUB(T4, T5));
	       Ta = VADD(T4, T5);
	  }
	  {
	       V Tc, Td, Tn, To;
	       Tc = LD(&(x[WS(vs, 1)]), dist, &(x[WS(vs, 1)]));
	       Td = LD(&(x[WS(vs, 1) + WS(is, 2)]), dist, &(x[WS(vs, 1)]));
	       Te = VSUB(Tc, Td);
	       Tk = VADD(Tc, Td);
	       Tn = LD(&(x[WS(vs, 2)]), dist, &(x[WS(vs, 2)]));
	       To = LD(&(x[WS(vs, 2) + WS(is, 2)]), dist, &(x[WS(vs, 2)]));
	       Tp = VSUB(Tn, To);
	       Tv = VADD(Tn, To);
	  }
	  {
	       V Tq, Tr, Tf, Tg;
	       Tq = LD(&(x[WS(vs, 2) + WS(is, 1)]), dist, &(x[WS(vs, 2) + WS(is, 1)]));
	       Tr = LD(&(x[WS(vs, 2) + WS(is, 3)]), dist, &(x[WS(vs, 2) + WS(is, 1)]));
	       Ts = VBYI(VSUB(Tq, Tr));
	       Tw = VADD(Tq, Tr);
	       Tf = LD(&(x[WS(vs, 1) + WS(is, 1)]), dist, &(x[WS(vs, 1) + WS(is, 1)]));
	       Tg = LD(&(x[WS(vs, 1) + WS(is, 3)]), dist, &(x[WS(vs, 1) + WS(is, 1)]));
	       Th = VBYI(VSUB(Tf, Tg));
	       Tl = VADD(Tf, Tg);
	  }
	  ST(&(x[0]), VADD(T9, Ta), dist, &(x[0]));
	  ST(&(x[WS(is, 1)]), VADD(Tk, Tl), dist, &(x[WS(is, 1)]));
	  ST(&(x[WS(is, 2)]), VADD(Tv, Tw), dist, &(x[0]));
	  ST(&(x[WS(is, 3)]), VADD(TG, TH), dist, &(x[WS(is, 1)]));
	  {
	       V T7, Ti, Tt, TE;
	       T7 = BYTW(&(W[TWVL * 4]), VSUB(T3, T6));
	       ST(&(x[WS(vs, 3)]), T7, dist, &(x[WS(vs, 3)]));
	       Ti = BYTW(&(W[TWVL * 4]), VSUB(Te, Th));
	       ST(&(x[WS(vs, 3) + WS(is, 1)]), Ti, dist, &(x[WS(vs, 3) + WS(is, 1)]));
	       Tt = BYTW(&(W[TWVL * 4]), VSUB(Tp, Ts));
	       ST(&(x[WS(vs, 3) + WS(is, 2)]), Tt, dist, &(x[WS(vs, 3)]));
	       TE = BYTW(&(W[TWVL * 4]), VSUB(TA, TD));
	       ST(&(x[WS(vs, 3) + WS(is, 3)]), TE, dist, &(x[WS(vs, 3) + WS(is, 1)]));
	  }
	  {
	       V T8, Tj, Tu, TF;
	       T8 = BYTW(&(W[0]), VADD(T3, T6));
	       ST(&(x[WS(vs, 1)]), T8, dist, &(x[WS(vs, 1)]));
	       Tj = BYTW(&(W[0]), VADD(Te, Th));
	       ST(&(x[WS(vs, 1) + WS(is, 1)]), Tj, dist, &(x[WS(vs, 1) + WS(is, 1)]));
	       Tu = BYTW(&(W[0]), VADD(Tp, Ts));
	       ST(&(x[WS(vs, 1) + WS(is, 2)]), Tu, dist, &(x[WS(vs, 1)]));
	       TF = BYTW(&(W[0]), VADD(TA, TD));
	       ST(&(x[WS(vs, 1) + WS(is, 3)]), TF, dist, &(x[WS(vs, 1) + WS(is, 1)]));
	  }
	  {
	       V Tb, Tm, Tx, TI;
	       Tb = BYTW(&(W[TWVL * 2]), VSUB(T9, Ta));
	       ST(&(x[WS(vs, 2)]), Tb, dist, &(x[WS(vs, 2)]));
	       Tm = BYTW(&(W[TWVL * 2]), VSUB(Tk, Tl));
	       ST(&(x[WS(vs, 2) + WS(is, 1)]), Tm, dist, &(x[WS(vs, 2) + WS(is, 1)]));
	       Tx = BYTW(&(W[TWVL * 2]), VSUB(Tv, Tw));
	       ST(&(x[WS(vs, 2) + WS(is, 2)]), Tx, dist, &(x[WS(vs, 2)]));
	       TI = BYTW(&(W[TWVL * 2]), VSUB(TG, TH));
	       ST(&(x[WS(vs, 2) + WS(is, 3)]), TI, dist, &(x[WS(vs, 2) + WS(is, 1)]));
	  }
     }
     END_SIMD();
     return W;
}

static const tw_instr twinstr[] = {
     VTW(1),
     VTW(2),
     VTW(3),
     {TW_NEXT, VL, 0}
};

static const ct_desc desc = { 4, "q1bv_4", twinstr, {44, 24, 0, 0}, &GENUS, 0, 0, 0 };

void X(codelet_q1bv_4) (planner *p) {
     X(kdft_difsq_register) (p, q1bv_4, &desc);
}