/* ../k1.f -- translated by f2c (version 20031025).
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

/* Subroutine */ int calck1_(real *arg, real *result, integer *jint)
{
    /* Initialized data */

    static real one = 1.f;
    static real g[3] = { -305.07151578787595807f,43117.653211351080007f,
	    -2706232.2985570842656f };
    static real pp[11] = { .064257745859173138767f,7.558458463117603081f,
	    131.82609918569941308f,810.94256146537402173f,
	    2312.374220916887155f,3454.0675585544584407f,
	    2859.0657697910288226f,1331.948643318322199f,
	    341.2295348680131291f,44.137176114230414036f,
	    2.2196792496874548962f };
    static real qq[9] = { 36.001069306861518855f,330.31020088765390854f,
	    1208.2692316002348638f,2118.100048717194381f,
	    1944.8440788918006154f,969.29165726802648634f,
	    259.51223655579051357f,34.552228452758912848f,
	    1.7710478032601086579f };
    static real zero = 0.f;
    static real xleast = 1.18e-38f;
    static real xsmall = 5.95e-8f;
    static real xinf = 3.4e38f;
    static real xmax = 85.343f;
    static real p[5] = { .4812707045687844231f,99.991373567429309922f,
	    7188.5382604084798576f,177333.2403514701563f,
	    719389.20065420586101f };
    static real q[3] = { -281.43915754538725829f,37264.298672067697862f,
	    -2214937.4878243304548f };
    static real f[5] = { -.2279559082695500239f,-53.103913335180275253f,
	    -4505.1623763436087023f,-147580.69205414222471f,
	    -1353116.1492785421328f };

    /* Builtin functions */
    double log(doublereal), exp(doublereal), sqrt(doublereal);

    /* Local variables */
    static integer i__;
    static real x, xx, sumf, sumg, sump, sumq;

/* -------------------------------------------------------------------- */

/* This packet computes modified Bessel functions of the second kind */
/*   and order one,  K1(X)  and  EXP(X)*K1(X), for real arguments X. */
/*   It contains two function type subprograms, BESK1  and  BESEK1, */
/*   and one subroutine type subprogram, CALCK1.  The calling */
/*   statements for the primary entries are */

/*                   Y=BESK1(X) */
/*   and */
/*                   Y=BESEK1(X) */

/*   where the entry points correspond to the functions K1(X) and */
/*   EXP(X)*K1(X), respectively.  The routine CALCK1 is intended */
/*   for internal packet use only, all computations within the */
/*   packet being concentrated in this routine.  The function */
/*   subprograms invoke CALCK1 with the statement */
/*          CALL CALCK1(ARG,RESULT,JINT) */
/*   where the parameter usage is as follows */

/*      Function                      Parameters for CALCK1 */
/*        Call             ARG                  RESULT          JINT */

/*     BESK1(ARG)  XLEAST .LT. ARG .LT. XMAX    K1(ARG)          1 */
/*     BESEK1(ARG)     XLEAST .LT. ARG       EXP(ARG)*K1(ARG)    2 */

/*   The main computation evaluates slightly modified forms of near */
/*   minimax rational approximations generated by Russon and Blair, */
/*   Chalk River (Atomic Energy of Canada Limited) Report AECL-3461, */
/*   1969.  This transportable program is patterned after the */
/*   machine-dependent FUNPACK packet NATSK1, but cannot match that */
/*   version for efficiency or accuracy.  This version uses rational */
/*   functions that theoretically approximate K-SUB-1(X) to at */
/*   least 18 significant decimal digits.  The accuracy achieved */
/*   depends on the arithmetic system, the compiler, the intrinsic */
/*   functions, and proper selection of the machine-dependent */
/*   constants. */

/* ******************************************************************* */
/* ******************************************************************* */

/* Explanation of machine-dependent constants */

/*   beta   = Radix for the floating-point system */
/*   minexp = Smallest representable power of beta */
/*   maxexp = Smallest power of beta that overflows */
/*   XLEAST = Smallest acceptable argument, i.e., smallest machine */
/*            number X such that 1/X is machine representable. */
/*   XSMALL = Argument below which BESK1(X) and BESEK1(X) may */
/*            each be represented by 1/X.  A safe value is the */
/*            largest X such that  1.0 + X = 1.0  to machine */
/*            precision. */
/*   XINF   = Largest positive machine number; approximately */
/*            beta**maxexp */
/*   XMAX   = Largest argument acceptable to BESK1;  Solution to */
/*            equation: */
/*               W(X) * (1+3/8X-15/128X**2) = beta**minexp */
/*            where  W(X) = EXP(-X)*SQRT(PI/2X) */


/*     Approximate values for some important machines are: */

/*                           beta       minexp       maxexp */

/*  CRAY-1        (S.P.)       2        -8193         8191 */
/*  Cyber 180/185 */
/*    under NOS   (S.P.)       2         -975         1070 */
/*  IEEE (IBM/XT, */
/*    SUN, etc.)  (S.P.)       2         -126          128 */
/*  IEEE (IBM/XT, */
/*    SUN, etc.)  (D.P.)       2        -1022         1024 */
/*  IBM 3033      (D.P.)      16          -65           63 */
/*  VAX D-Format  (D.P.)       2         -128          127 */
/*  VAX G-Format  (D.P.)       2        -1024         1023 */


/*                         XLEAST     XSMALL      XINF       XMAX */

/* CRAY-1                1.84E-2466  3.55E-15  5.45E+2465  5674.858 */
/* Cyber 180/855 */
/*   under NOS   (S.P.)  3.14E-294   1.77E-15  1.26E+322    672.789 */
/* IEEE (IBM/XT, */
/*   SUN, etc.)  (S.P.)  1.18E-38    5.95E-8   3.40E+38      85.343 */
/* IEEE (IBM/XT, */
/*   SUN, etc.)  (D.P.)  2.23D-308   1.11D-16  1.79D+308    705.343 */
/* IBM 3033      (D.P.)  1.39D-76    1.11D-16  7.23D+75     177.855 */
/* VAX D-Format  (D.P.)  5.88D-39    6.95D-18  1.70D+38      86.721 */
/* VAX G-Format  (D.P.)  1.12D-308   5.55D-17  8.98D+307    706.728 */

/* ******************************************************************* */
/* ******************************************************************* */

/* Error returns */

/*  The program returns the value XINF for ARG .LE. 0.0 and the */
/*   BESK1 entry returns the value 0.0 for ARG .GT. XMAX. */


/*  Intrinsic functions required are: */

/*     LOG, SQRT, EXP */


/*  Authors: W. J. Cody and Laura Stoltz */
/*           Mathematics and Computer Science Division */
/*           Argonne National Laboratory */
/*           Argonne, IL 60439 */

/*  Latest modification: January 28, 1988 */

/* -------------------------------------------------------------------- */
/* -------------------------------------------------------------------- */
/*  Mathematical constants */
/* -------------------------------------------------------------------- */
/* -------------------------------------------------------------------- */
/*  Machine-dependent constants */
/* -------------------------------------------------------------------- */
/* -------------------------------------------------------------------- */
/*  Coefficients for  XLEAST .LE.  ARG  .LE. 1.0 */
/* -------------------------------------------------------------------- */
/* -------------------------------------------------------------------- */
/*  Coefficients for  1.0 .LT.  ARG */
/* -------------------------------------------------------------------- */
/* -------------------------------------------------------------------- */
    x = *arg;
    if (x < xleast) {
/* -------------------------------------------------------------------- */
/*  Error return for  ARG  .LT. XLEAST */
/* -------------------------------------------------------------------- */
	*result = xinf;
    } else if (x <= one) {
/* -------------------------------------------------------------------- */
/*  XLEAST .LE.  ARG  .LE. 1.0 */
/* -------------------------------------------------------------------- */
	if (x < xsmall) {
/* -------------------------------------------------------------------- */
/*  Return for small ARG */
/* -------------------------------------------------------------------- */
	    *result = one / x;
	} else {
	    xx = x * x;
	    sump = ((((p[0] * xx + p[1]) * xx + p[2]) * xx + p[3]) * xx + p[4]
		    ) * xx + q[2];
	    sumq = ((xx + q[0]) * xx + q[1]) * xx + q[2];
	    sumf = (((f[0] * xx + f[1]) * xx + f[2]) * xx + f[3]) * xx + f[4];
	    sumg = ((xx + g[0]) * xx + g[1]) * xx + g[2];
	    *result = (xx * log(x) * sumf / sumg + sump / sumq) / x;
	    if (*jint == 2) {
		*result *= exp(x);
	    }
	}
    } else if (*jint == 1 && x > xmax) {
/* -------------------------------------------------------------------- */
/*  Error return for  ARG  .GT. XMAX */
/* -------------------------------------------------------------------- */
	*result = zero;
    } else {
/* -------------------------------------------------------------------- */
/*  1.0 .LT.  ARG */
/* -------------------------------------------------------------------- */
	xx = one / x;
	sump = pp[0];
	for (i__ = 2; i__ <= 11; ++i__) {
	    sump = sump * xx + pp[i__ - 1];
/* L120: */
	}
	sumq = xx;
	for (i__ = 1; i__ <= 8; ++i__) {
	    sumq = (sumq + qq[i__ - 1]) * xx;
/* L140: */
	}
	sumq += qq[8];
	*result = sump / sumq / sqrt(x);
	if (*jint == 1) {
	    *result *= exp(-x);
	}
    }
    return 0;
/* ---------- Last line of CALCK1 ---------- */
} /* calck1_ */

doublereal besk1_(real *x)
{
    /* System generated locals */
    real ret_val;

    /* Local variables */
    static integer jint;
    extern /* Subroutine */ int calck1_(real *, real *, integer *);
    static real result;

/* -------------------------------------------------------------------- */

/* This function program computes approximate values for the */
/*   modified Bessel function of the second kind of order one */
/*   for arguments  XLEAST .LE. ARG .LE. XMAX. */

/* -------------------------------------------------------------------- */
/* -------------------------------------------------------------------- */
    jint = 1;
    calck1_(x, &result, &jint);
    ret_val = result;
    return ret_val;
/* ---------- Last line of BESK1 ---------- */
} /* besk1_ */

doublereal besek1_(real *x)
{
    /* System generated locals */
    real ret_val;

    /* Local variables */
    static integer jint;
    extern /* Subroutine */ int calck1_(real *, real *, integer *);
    static real result;

/* -------------------------------------------------------------------- */

/* This function program computes approximate values for the */
/*   modified Bessel function of the second kind of order one */
/*   multiplied by the exponential function, for arguments */
/*   XLEAST .LE. ARG .LE. XMAX. */

/* -------------------------------------------------------------------- */
/* -------------------------------------------------------------------- */
    jint = 2;
    calck1_(x, &result, &jint);
    ret_val = result;
    return ret_val;
/* ---------- Last line of BESEK1 ---------- */
} /* besek1_ */
