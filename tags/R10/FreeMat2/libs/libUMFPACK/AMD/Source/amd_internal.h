/* ========================================================================= */
/* === amd_internal.h ====================================================== */
/* ========================================================================= */

/* ------------------------------------------------------------------------- */
/* AMD Version 1.1 (Jan. 21, 2004), Copyright (c) 2004 by Timothy A. Davis,  */
/* Patrick R. Amestoy, and Iain S. Duff.  See ../README for License.         */
/* email: davis@cise.ufl.edu    CISE Department, Univ. of Florida.           */
/* web: http://www.cise.ufl.edu/research/sparse/amd                          */
/* ------------------------------------------------------------------------- */

/* This file is for internal use in AMD itself, and does not normally need to
 * be included in user code.  Use amd.h instead.
 *
 * The following compile-time definitions affect how AMD is compiled.
 *
 *	-DMATLAB_MEX_FILE
 *
 *	    This flag is turned on when compiling the amd mexFunction for
 *	    use in MATLAB.
 *
 *	-DMATHWORKS
 *
 *	    This flag is turned on when compiling amd as a built-in routine
 *	    in MATLAB.  Internal routines utMalloc, utFree, utRealloc, and
 *	    utPrintf are used, and the MathWorks "util.h" file is included.  
 *	    This option is intended for use by The MathWorks, Inc., only.
 *
 *	-DNDEBUG
 *
 *	    Debugging mode (if NDEBUG is not defined).  The default, of course,
 *	    is no debugging.  Turning on debugging takes some work (see below).
 *	    If you do not edit this file, then debugging is turned off anyway,
 *	    regardless of whether or not -DNDEBUG is specified in your compiler
 *	    options.
 *
 *	-DALLOCATE=allocation_routine
 *	-DFREE=free_routine
 *
 *	    If you do not wish to use malloc or free, you can define the
 *	    routines to be used here.  You must specify both of them, or
 *	    neither.
 *
 *	-DPRINTF=printf_routine
 *
 *	    If you wish to use a routine other than printf, you can define it
 *	    with -DPRINTF= followed by the name of the printf replacement.
 */

/* ========================================================================= */
/* === NDEBUG ============================================================== */
/* ========================================================================= */

/*
    AMD will be exceedingly slow when running in debug mode.  The next three
    lines ensure that debugging is turned off.
*/
#ifndef NDEBUG
#define NDEBUG
#endif

/*
    To enable debugging, uncomment the following line:
#undef NDEBUG
*/

/* ------------------------------------------------------------------------- */
/* ANSI include files */
/* ------------------------------------------------------------------------- */

/* from stdlib.h:  malloc, free, realloc (when not compiling for MATLAB) */
#include <stdlib.h>

/* from stdio.h:  printf.  When in debug mode:  fopen, fscanf */
#include <stdio.h>

/* from limits.h:  INT_MAX and LONG_MAX */
#include <limits.h>

/* from math.h: sqrt */
#include <math.h>

/* ------------------------------------------------------------------------- */
/* MATLAB include files (only if being used in or via MATLAB) */
/* ------------------------------------------------------------------------- */

#ifdef MATHWORKS
#include "util.h"
#endif

#ifdef MATLAB_MEX_FILE
#include "matrix.h"
#include "mex.h"
#endif

/* ------------------------------------------------------------------------- */
/* basic definitions */
/* ------------------------------------------------------------------------- */

#ifdef FLIP
#undef FLIP
#endif

#ifdef MAX
#undef MAX
#endif

#ifdef MIN
#undef MIN
#endif

#ifdef EMPTY
#undef EMPTY
#endif

#ifdef GLOBAL
#undef GLOBAL
#endif

#ifdef PRIVATE
#undef PRIVATE
#endif

/* FLIP is a "negation about -1", and is used to mark an integer i that is
 * normally non-negative.  FLIP (EMPTY) is EMPTY.  FLIP of a number > EMPTY
 * is negative, and FLIP of a number < EMTPY is positive.  FLIP (FLIP (i)) = i
 * for all integers i.  UNFLIP (i) is >= EMPTY. */
#define EMPTY (-1)
#define FLIP(i) (-(i)-2)
#define UNFLIP(i) ((i < EMPTY) ? FLIP (i) : (i))

/* for integer MAX/MIN, or for doubles when we don't care how NaN's behave: */
#define MAX(a,b) (((a) > (b)) ? (a) : (b))
#define MIN(a,b) (((a) < (b)) ? (a) : (b))

/* logical expression of p implies q: */
#define IMPLIES(p,q) (!(p) || (q))

/* Note that the IBM RS 6000 xlc predefines TRUE and FALSE in <types.h>. */
/* The Compaq Alpha also predefines TRUE and FALSE. */
#ifdef TRUE
#undef TRUE
#endif
#ifdef FALSE
#undef FALSE
#endif

#define TRUE (1)
#define FALSE (0)
#define PRIVATE static
#define GLOBAL
#define EMPTY (-1)

/* Note that Linux's gcc 2.96 defines NULL as ((void *) 0), but other */
/* compilers (even gcc 2.95.2 on Solaris) define NULL as 0 or (0).  We */
/* need to use the ANSI standard value of 0. */
#ifdef NULL
#undef NULL
#endif

#define NULL 0

/* ------------------------------------------------------------------------- */
/* integer type for AMD: int or long */
/* ------------------------------------------------------------------------- */

#if defined (DLONG) || defined (ZLONG)

#define Int long
#define ID "%ld"
#define Int_MAX LONG_MAX
#define Int_MIN LONG_MIN

#define AMD_order amd_l_order
#define AMD_defaults amd_l_defaults
#define AMD_control amd_l_control
#define AMD_info amd_l_info
#define AMD_1 amd_l1
#define AMD_2 amd_l2
#define AMD_valid amd_l_valid
#define AMD_aat amd_l_aat
#define AMD_postorder amd_l_postorder
#define AMD_post_tree amd_l_post_tree
#define AMD_dump amd_l_dump
#define AMD_debug amd_l_debug
#define AMD_debug_init amd_l_debug_init
#define AMD_wpreprocess amd_l_wpreprocess
#define AMD_preprocess amd_l_preprocess
#define AMD_preprocess_valid amd_l_preprocess_valid

#else

#define Int int
#define ID "%d"
#define Int_MAX INT_MAX
#define Int_MIN INT_MIN

#define AMD_order amd_order
#define AMD_defaults amd_defaults
#define AMD_control amd_control
#define AMD_info amd_info
#define AMD_1 amd_1
#define AMD_2 amd_2
#define AMD_valid amd_valid
#define AMD_aat amd_aat
#define AMD_postorder amd_postorder
#define AMD_post_tree amd_post_tree
#define AMD_dump amd_dump
#define AMD_debug amd_debug
#define AMD_debug_init amd_debug_init
#define AMD_wpreprocess amd_wpreprocess
#define AMD_preprocess amd_preprocess
#define AMD_preprocess_valid amd_preprocess_valid

#endif

/* ========================================================================= */
/* === Memory allocator ==================================================== */
/* ========================================================================= */

/* The MATLAB mexFunction uses MATLAB's memory manager, while the C-callable */
/* AMD routine uses the ANSI C malloc, free, and realloc routines. */

#ifndef ALLOCATE
#ifdef MATLAB_MEX_FILE
#define ALLOCATE mxMalloc
#define FREE mxFree
#else
#ifdef MATHWORKS
/* Compiling as a built-in routine.  Since out-of-memory conditions are checked
 * after every allocation, we can use ut* routines here. */
#define ALLOCATE utMalloc
#define FREE utFree
#else
/* use the ANSI C memory allocation routines */
#define ALLOCATE malloc
#define FREE free
#endif
#endif
#endif


/* ========================================================================= */
/* === PRINTF macro ======================================================== */
/* ========================================================================= */

/* All output goes through the PRINTF macro.  */

#ifndef PRINTF
#ifdef MATLAB_MEX_FILE
#define PRINTF(params) { (void) mexPrintf params ; }
#else
#ifdef MATHWORKS
#define PRINTF(params) { (void) utPrintf params ; }
#else
#define PRINTF(params) { (void) printf params ; }
#endif
#endif
#endif

/* ------------------------------------------------------------------------- */
/* AMD routine definitions (user-callable) */
/* ------------------------------------------------------------------------- */

#include "amd.h"

/* ------------------------------------------------------------------------- */
/* AMD routine definitions (not user-callable) */
/* ------------------------------------------------------------------------- */

GLOBAL Int AMD_valid
(
    Int n_row,
    Int n_col,
    const Int Ap [ ],
    const Int Ai [ ]
) ;

GLOBAL Int AMD_aat
(
    Int n,
    const Int Ap [ ],
    const Int Ai [ ],
    Int Len [ ],
    Int Tp [ ],	
    double Info [ ]
) ;

GLOBAL void AMD_1
(
    Int n,
    const Int Ap [ ],
    const Int Ai [ ],
    Int P [ ],
    Int Pinv [ ],
    Int Len [ ],
    Int slen,
    Int S [ ],
    double Control [ ],
    double Info [ ]
) ;

GLOBAL void AMD_2 (
    Int n,
    Int Pe [ ],
    Int Iw [ ],
    Int Len [ ],
    Int iwlen,
    Int pfree,
    Int Nv [ ],
    Int Next [ ], 
    Int Last [ ],
    Int Head [ ],
    Int Elen [ ],
    Int Degree [ ],
    Int W [ ],
    double Control [ ],
    double Info [ ]
) ;

GLOBAL void AMD_postorder
(
    Int nn,
    Int Parent [ ],
    Int Npiv [ ],
    Int Fsize [ ],
    Int Order [ ],
    Int Child [ ],
    Int Sibling [ ],
    Int Stack [ ]
) ;

GLOBAL Int AMD_post_tree
(
    Int root,
    Int k,
    Int Child [ ],
    const Int Sibling [ ],
    Int Order [ ],
    Int Stack [ ]
#ifndef NDEBUG
    , Int nn
#endif
) ;

GLOBAL void AMD_wpreprocess
(
    Int n,
    const Int Ap [ ],
    const Int Ai [ ],
    Int Rp [ ],
    Int Ri [ ],
    Int W [ ],
    Int Flag [ ]
) ;

GLOBAL Int AMD_preprocess_valid
(
    Int n,
    const Int Ap [ ],
    const Int Ai [ ]
) ;

/* ------------------------------------------------------------------------- */
/* debugging definitions */
/* ------------------------------------------------------------------------- */

/* from assert.h:  assert macro */
#if !defined (MATHWORKS) && !defined (MATLAB_MEX_FILE)
#include <assert.h>
#endif

#ifndef NDEBUG

GLOBAL Int AMD_debug ;

GLOBAL void AMD_debug_init ( char *s ) ;

GLOBAL void AMD_dump (
    Int n,
    Int Pe [ ],
    Int Iw [ ],
    Int Len [ ],
    Int iwlen,
    Int pfree,
    Int Nv [ ],
    Int Next [ ],
    Int Last [ ],
    Int Head [ ],
    Int Elen [ ],
    Int Degree [ ],
    Int W [ ],
    Int nel
) ;

#ifdef ASSERT
#undef ASSERT
#endif

#ifdef MATLAB_MEX_FILE
#define ASSERT(expression) (mxAssert ((expression), ""))
#else
#ifdef MATHWORKS
#define ASSERT(expression) (utAssert (expression))
#else
#define ASSERT(expression) (assert (expression))
#endif
#endif /* MATLAB_MEX_FILE */

#define AMD_DEBUG0(params) { PRINTF (params) ; }
#define AMD_DEBUG1(params) { if (AMD_debug >= 1) PRINTF (params) ; }
#define AMD_DEBUG2(params) { if (AMD_debug >= 2) PRINTF (params) ; }
#define AMD_DEBUG3(params) { if (AMD_debug >= 3) PRINTF (params) ; }
#define AMD_DEBUG4(params) { if (AMD_debug >= 4) PRINTF (params) ; }

#else

#define AMD_DEBUG0(params)
#define AMD_DEBUG1(params)
#define AMD_DEBUG2(params)
#define AMD_DEBUG3(params)
#define AMD_DEBUG4(params)

#define ASSERT(expression)

#endif