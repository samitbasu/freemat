/* ../i1.f -- translated by f2c (version 20031025).
   You must link the resulting object file with libf2c:
	on Microsoft Windows system, link with libf2c.lib;
	on Linux or Unix systems, link with .../path/to/libf2c.a -lm
	or, if you install libf2c.a in a standard place, with -lf2c -lm
	-- in that order, at the end of the command line, as in
		cc *.o -lf2c -lm
	Source for libf2c is in /netlib/f2c/libf2c.zip, e.g.,

		http://www.netlib.org/f2c/libf2c.zip
*/

#include "f2c.h"

/* Subroutine */ int calci1_(real *arg, real *result, integer *jint)
{
    /* Initialized data */

    static real one = 1.f;
    static real xinf = 3.4e38f;
    static real xmax = 91.906f;
    static real p[15] = { -1.970529180253513993e-19f,
	    -6.524551558315190291e-16f,-1.1928788903603238754e-12f,
	    -1.4831904935994647675e-9f,-1.3466829827635152875e-6f,
	    -9.1746443287817501309e-4f,-.47207090827310162436f,
	    -182.25946631657315931f,-51894.09198230801754f,
	    -10588550.724769347106f,-1482826760.6612366099f,
	    -133574376822.75493024f,-6987677964801.009007f,
	    -177320378407915.9132f,-1457718027814346.3643f };
    static real q[5] = { -4007.6864679904189921f,7481058.0356655069138f,
	    -8005951899.8619764991f,4854471425827.3622913f,
	    -1321816830732144.2305f };
    static real pp[8] = { -.0604371590561376f,.45748122901933459f,
	    -.42843766903304806403f,.097356000150886612134f,
	    -.0032457723974465568321f,-3.6395264712121795296e-4f,
	    1.6258661867440836395e-5f,-3.6347578404608223492e-7f };
    static real qq[6] = { -3.880658672155659345f,3.2593714889036996297f,
	    -.85017476463217924408f,.074212010813186530069f,
	    -.0022835624489492512649f,3.7510433111922824643e-5f };
    static real pbar = .3984375f;
    static real one5 = 15.f;
    static real exp40 = 235385266837019985.4f;
    static real forty = 40.f;
    static real rec15 = .066666666666666666666f;
    static real two25 = 225.f;
    static real half = .5f;
    static real zero = 0.f;
    static real xsmall = 2.98e-8f;

    /* Builtin functions */
    double exp(doublereal), sqrt(doublereal);

    /* Local variables */
    static real a, b;
    static integer j;
    static real x, xx, sump, sumq;

/* -------------------------------------------------------------------- */

/* This packet computes modified Bessel functioons of the first kind */
/*   and order one, I1(X) and EXP(-ABS(X))*I1(X), for real */
/*   arguments X.  It contains two function type subprograms, BESI1 */
/*   and BESEI1, and one subroutine type subprogram, CALCI1. */
/*   The calling statements for the primary entries are */

/*                   Y=BESI1(X) */
/*   and */
/*                   Y=BESEI1(X) */

/*   where the entry points correspond to the functions I1(X) and */
/*   EXP(-ABS(X))*I1(X), respectively.  The routine CALCI1 is */
/*   intended for internal packet use only, all computations within */
/*   the packet being concentrated in this routine.  The function */
/*   subprograms invoke CALCI1 with the statement */
/*          CALL CALCI1(ARG,RESULT,JINT) */
/*   where the parameter usage is as follows */

/*      Function                     Parameters for CALCI1 */
/*       Call              ARG                  RESULT          JINT */

/*     BESI1(ARG)    ABS(ARG) .LE. XMAX        I1(ARG)           1 */
/*     BESEI1(ARG)    any real ARG        EXP(-ABS(ARG))*I1(ARG) 2 */

/*   The main computation evaluates slightly modified forms of */
/*   minimax approximations generated by Blair and Edwards, Chalk */
/*   River (Atomic Energy of Canada Limited) Report AECL-4928, */
/*   October, 1974.  This transportable program is patterned after */
/*   the machine-dependent FUNPACK packet NATSI1, but cannot match */
/*   that version for efficiency or accuracy.  This version uses */
/*   rational functions that theoretically approximate I-SUB-1(X) */
/*   to at least 18 significant decimal digits.  The accuracy */
/*   achieved depends on the arithmetic system, the compiler, the */
/*   intrinsic functions, and proper selection of the machine- */
/*   dependent constants. */

/* ******************************************************************* */
/* ******************************************************************* */

/* Explanation of machine-dependent constants */

/*   beta   = Radix for the floating-point system */
/*   maxexp = Smallest power of beta that overflows */
/*   XSMALL = Positive argument such that 1.0 - X = 1.0 to */
/*            machine precision for all ABS(X) .LE. XSMALL. */
/*   XINF =   Largest positive machine number; approximately */
/*            beta**maxexp */
/*   XMAX =   Largest argument acceptable to BESI1;  Solution to */
/*            equation: */
/*               EXP(X) * (1-3/(8*X)) / SQRT(2*PI*X) = beta**maxexp */


/*     Approximate values for some important machines are: */

/*                          beta       maxexp       XSMALL */

/* CRAY-1        (S.P.)       2         8191       3.55E-15 */
/* Cyber 180/855 */
/*   under NOS   (S.P.)       2         1070       3.55E-15 */
/* IEEE (IBM/XT, */
/*   SUN, etc.)  (S.P.)       2          128       2.98E-8 */
/* IEEE (IBM/XT, */
/*   SUN, etc.)  (D.P.)       2         1024       5.55D-17 */
/* IBM 3033      (D.P.)      16           63       6.95D-18 */
/* VAX           (S.P.)       2          127       2.98E-8 */
/* VAX D-Format  (D.P.)       2          127       6.95D-18 */
/* VAX G-Format  (D.P.)       2         1023       5.55D-17 */


/*                               XINF          XMAX */

/* CRAY-1        (S.P.)       5.45E+2465     5682.810 */
/* Cyber 180/855 */
/*   under NOS   (S.P.)       1.26E+322       745.894 */
/* IEEE (IBM/XT, */
/*   SUN, etc.)  (S.P.)       3.40E+38         91.906 */
/* IEEE (IBM/XT, */
/*   SUN, etc.)  (D.P.)       1.79D+308       713.987 */
/* IBM 3033      (D.P.)       7.23D+75        178.185 */
/* VAX           (S.P.)       1.70D+38         91.209 */
/* VAX D-Format  (D.P.)       1.70D+38         91.209 */
/* VAX G-Format  (D.P.)       8.98D+307       713.293 */

/* ******************************************************************* */
/* ******************************************************************* */

/* Error returns */

/*  The program returns the value XINF for ABS(ARG) .GT. XMAX. */


/* Intrinsic functions required are: */

/*     ABS, SQRT, EXP */


/*  Authors: W. J. Cody and L. Stoltz */
/*           Mathematics and Computer Science Division */
/*           Argonne National Laboratory */
/*           Argonne, IL  60439 */

/*  Latest modification: May 31, 1989 */

/* -------------------------------------------------------------------- */
/* -------------------------------------------------------------------- */
/*  Mathematical constants */
/* -------------------------------------------------------------------- */
/* -------------------------------------------------------------------- */
/*  Machine-dependent constants */
/* -------------------------------------------------------------------- */
/* -------------------------------------------------------------------- */
/*  Coefficients for XSMALL .LE. ABS(ARG) .LT. 15.0 */
/* -------------------------------------------------------------------- */
/* -------------------------------------------------------------------- */
/*  Coefficients for 15.0 .LE. ABS(ARG) */
/* -------------------------------------------------------------------- */
/* -------------------------------------------------------------------- */
    x = dabs(*arg);
    if (x < xsmall) {
/* -------------------------------------------------------------------- */
/*  Return for ABS(ARG) .LT. XSMALL */
/* -------------------------------------------------------------------- */
	*result = half * x;
    } else if (x < one5) {
/* -------------------------------------------------------------------- */
/*  XSMALL .LE. ABS(ARG) .LT. 15.0 */
/* -------------------------------------------------------------------- */
	xx = x * x;
	sump = p[0];
	for (j = 2; j <= 15; ++j) {
	    sump = sump * xx + p[j - 1];
/* L50: */
	}
	xx -= two25;
	sumq = ((((xx + q[0]) * xx + q[1]) * xx + q[2]) * xx + q[3]) * xx + q[
		4];
	*result = sump / sumq * x;
	if (*jint == 2) {
	    *result *= exp(-x);
	}
    } else if (*jint == 1 && x > xmax) {
	*result = xinf;
    } else {
/* -------------------------------------------------------------------- */
/*  15.0 .LE. ABS(ARG) */
/* -------------------------------------------------------------------- */
	xx = one / x - rec15;
	sump = ((((((pp[0] * xx + pp[1]) * xx + pp[2]) * xx + pp[3]) * xx + 
		pp[4]) * xx + pp[5]) * xx + pp[6]) * xx + pp[7];
	sumq = (((((xx + qq[0]) * xx + qq[1]) * xx + qq[2]) * xx + qq[3]) * 
		xx + qq[4]) * xx + qq[5];
	*result = sump / sumq;
	if (*jint != 1) {
	    *result = (*result + pbar) / sqrt(x);
	} else {
/* -------------------------------------------------------------------- */
/*  Calculation reformulated to avoid premature overflow */
/* -------------------------------------------------------------------- */
	    if (x > xmax - one5) {
		a = exp(x - forty);
		b = exp40;
	    } else {
		a = exp(x);
		b = one;
	    }
	    *result = (*result * a + pbar * a) / sqrt(x) * b;
/* -------------------------------------------------------------------- */
/*  Error return for ABS(ARG) .GT. XMAX */
/* -------------------------------------------------------------------- */
	}
    }
    if (*arg < zero) {
	*result = -(*result);
    }
    return 0;
/* ----------- Last line of CALCI1 ----------- */
} /* calci1_ */

doublereal besi1_(real *x)
{
    /* System generated locals */
    real ret_val;

    /* Local variables */
    static integer jint;
    extern /* Subroutine */ int calci1_(real *, real *, integer *);
    static real result;

/* -------------------------------------------------------------------- */

/* This long precision subprogram computes approximate values for */
/*   modified Bessel functions of the first kind of order one for */
/*   arguments ABS(ARG) .LE. XMAX  (see comments heading CALCI1). */

/* -------------------------------------------------------------------- */
/* -------------------------------------------------------------------- */
    jint = 1;
    calci1_(x, &result, &jint);
    ret_val = result;
    return ret_val;
/* ---------- Last line of BESI1 ---------- */
} /* besi1_ */

doublereal besei1_(real *x)
{
    /* System generated locals */
    real ret_val;

    /* Local variables */
    static integer jint;
    extern /* Subroutine */ int calci1_(real *, real *, integer *);
    static real result;

/* -------------------------------------------------------------------- */

/* This function program computes approximate values for the */
/*   modified Bessel function of the first kind of order one */
/*   multiplied by EXP(-ABS(X)), where EXP is the */
/*   exponential function, ABS is the absolute value, and X */
/*   is any argument. */

/* -------------------------------------------------------------------- */
/* -------------------------------------------------------------------- */
    jint = 2;
    calci1_(x, &result, &jint);
    ret_val = result;
    return ret_val;
/* ---------- Last line of BESEI1 ---------- */
} /* besei1_ */

