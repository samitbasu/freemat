      FUNCTION DDAW(XX)
C----------------------------------------------------------------------
C
C This function program evaluates Dawson's integral, 
C
C                       2  / x   2
C                     -x   |    t
C             F(x) = e     |   e    dt
C                          |
C                          / 0
C
C   for a real argument x.
C
C   The calling sequence for this function is 
C
C                   Y=DAW(X)
C
C   The main computation uses rational Chebyshev approximations
C   published in Math. Comp. 24, 171-178 (1970) by Cody, Paciorek
C   and Thacher.  This transportable program is patterned after the
C   machine-dependent FUNPACK program DDAW(X), but cannot match that
C   version for efficiency or accuracy.  This version uses rational
C   approximations that are theoretically accurate to about 19
C   significant decimal digits.  The accuracy achieved depends on the
C   arithmetic system, the compiler, the intrinsic functions, and
C   proper selection of the machine-dependent constants.
C
C*******************************************************************
C*******************************************************************
C
C Explanation of machine-dependent constants
C
C   XINF   = largest positive machine number
C   XMIN   = the smallest positive machine number.
C   EPS    = smallest positive number such that 1+eps > 1.
C            Approximately  beta**(-p), where beta is the machine
C            radix and p is the number of significant base-beta
C            digits in a floating-point number.
C   XMAX   = absolute argument beyond which DAW(X) underflows.
C            XMAX = min(0.5/xmin, xinf).
C   XSMALL = absolute argument below DAW(X)  may be represented
C            by X.  We recommend XSMALL = sqrt(eps).
C   XLARGE = argument beyond which DAW(X) may be represented by
C            1/(2x).  We recommend XLARGE = 1/sqrt(eps).
C
C     Approximate values for some important machines are
C
C                        beta  p     eps     xmin       xinf  
C
C  CDC 7600      (S.P.)    2  48  7.11E-15  3.14E-294  1.26E+322
C  CRAY-1        (S.P.)    2  48  7.11E-15  4.58E-2467 5.45E+2465
C  IEEE (IBM/XT,
C    SUN, etc.)  (S.P.)    2  24  1.19E-07  1.18E-38   3.40E+38
C  IEEE (IBM/XT,
C    SUN, etc.)  (D.P.)    2  53  1.11D-16  2.23E-308  1.79D+308
C  IBM 3033      (D.P.)   16  14  1.11D-16  5.40D-79   7.23D+75
C  VAX 11/780    (S.P.)    2  24  5.96E-08  2.94E-39   1.70E+38
C                (D.P.)    2  56  1.39D-17  2.94D-39   1.70D+38
C   (G Format)   (D.P.)    2  53  1.11D-16  5.57D-309  8.98D+307
C
C                         XSMALL     XLARGE     XMAX    
C
C  CDC 7600      (S.P.)  5.96E-08   1.68E+07  1.59E+293
C  CRAY-1        (S.P.)  5.96E-08   1.68E+07  5.65E+2465
C  IEEE (IBM/XT,
C    SUN, etc.)  (S.P.)  2.44E-04   4.10E+03  4.25E+37
C  IEEE (IBM/XT,
C    SUN, etc.)  (D.P.)  1.05E-08   9.49E+07  2.24E+307
C  IBM 3033      (D.P.)  3.73D-09   2.68E+08  7.23E+75
C  VAX 11/780    (S.P.)  2.44E-04   4.10E+03  1.70E+38
C                (D.P.)  3.73E-09   2.68E+08  1.70E+38
C   (G Format)   (D.P.)  1.05E-08   9.49E+07  8.98E+307
C
C*******************************************************************
C*******************************************************************
C
C Error Returns
C
C  The program returns 0.0 for |X| > XMAX.
C
C Intrinsic functions required are:
C
C     ABS
C
C
C  Author: W. J. Cody
C          Mathematics and Computer Science Division 
C          Argonne National Laboratory
C          Argonne, IL 60439
C
C  Latest modification: June 15, 1988
C
C----------------------------------------------------------------------
      INTEGER I
      DOUBLE PRECISION
     1     DDAW,FRAC,HALF,ONE,ONE225,P1,P2,P3,P4,Q1,Q2,Q3,Q4,SIX25,
     2     SUMP,SUMQ,TWO5,W2,X,XX,Y,XLARGE,XMAX,XSMALL,ZERO
      DIMENSION P1(10),P2(10),P3(10),P4(10),Q1(10),Q2(9),Q3(9),Q4(9)
C----------------------------------------------------------------------
C  Mathematical constants.
C----------------------------------------------------------------------
      DATA ZERO,HALF,ONE/0.0D0,0.5D0,1.0D0/,
     1     SIX25,ONE225,TWO5/6.25D0,12.25D0,25.0D0/
C----------------------------------------------------------------------
C  Machine-dependent constants
C----------------------------------------------------------------------
      DATA XSMALL/1.05D-08/, XLARGE/9.49D+07/, XMAX/2.24D+307/
C----------------------------------------------------------------------
C  Coefficients for R(9,9) approximation for  |x| < 2.5
C----------------------------------------------------------------------
      DATA P1/-2.69020398788704782410D-12, 4.18572065374337710778D-10,
     1        -1.34848304455939419963D-08, 9.28264872583444852976D-07,
     2        -1.23877783329049120592D-05, 4.07205792429155826266D-04,
     3        -2.84388121441008500446D-03, 4.70139022887204722217D-02,
     4        -1.38868086253931995101D-01, 1.00000000000000000004D+00/
      DATA Q1/ 1.71257170854690554214D-10, 1.19266846372297253797D-08,
     1         4.32287827678631772231D-07, 1.03867633767414421898D-05,
     2         1.78910965284246249340D-04, 2.26061077235076703171D-03,
     3         2.07422774641447644725D-02, 1.32212955897210128811D-01,
     4         5.27798580412734677256D-01, 1.00000000000000000000D+00/
C----------------------------------------------------------------------
C  Coefficients for R(9,9) approximation in J-fraction form
C     for  x in [2.5, 3.5)
C----------------------------------------------------------------------
      DATA P2/-1.70953804700855494930D+00,-3.79258977271042880786D+01,
     1         2.61935631268825992835D+01, 1.25808703738951251885D+01,
     2        -2.27571829525075891337D+01, 4.56604250725163310122D+00,
     3        -7.33080089896402870750D+00, 4.65842087940015295573D+01,
     4        -1.73717177843672791149D+01, 5.00260183622027967838D-01/
      DATA Q2/ 1.82180093313514478378D+00, 1.10067081034515532891D+03,
     1        -7.08465686676573000364D+00, 4.53642111102577727153D+02,
     2         4.06209742218935689922D+01, 3.02890110610122663923D+02,
     3         1.70641269745236227356D+02, 9.51190923960381458747D+02,
     4         2.06522691539642105009D-01/
C----------------------------------------------------------------------
C  Coefficients for R(9,9) approximation in J-fraction form
C     for  x in [3.5, 5.0]
C----------------------------------------------------------------------
      DATA P3/-4.55169503255094815112D+00,-1.86647123338493852582D+01,
     1        -7.36315669126830526754D+00,-6.68407240337696756838D+01,
     2         4.84507265081491452130D+01, 2.69790586735467649969D+01,
     3        -3.35044149820592449072D+01, 7.50964459838919612289D+00,
     4        -1.48432341823343965307D+00, 4.99999810924858824981D-01/
      DATA Q3/ 4.47820908025971749852D+01, 9.98607198039452081913D+01,
     1         1.40238373126149385228D+01, 3.48817758822286353588D+03,
     2        -9.18871385293215873406D+00, 1.24018500009917163023D+03,
     3        -6.88024952504512254535D+01,-2.31251575385145143070D+00,
     4         2.50041492369922381761D-01/
C----------------------------------------------------------------------
C  Coefficients for R(9,9) approximation in J-fraction form
C     for  |x| > 5.0
C----------------------------------------------------------------------
      DATA P4/-8.11753647558432685797D+00,-3.84043882477454453430D+01,
     1        -2.23787669028751886675D+01,-2.88301992467056105854D+01,
     2        -5.99085540418222002197D+00,-1.13867365736066102577D+01,
     3        -6.52828727526980741590D+00,-4.50002293000355585708D+00,
     4        -2.50000000088955834952D+00, 5.00000000000000488400D-01/
      DATA Q4/ 2.69382300417238816428D+02, 5.04198958742465752861D+01,
     1         6.11539671480115846173D+01, 2.08210246935564547889D+02,
     2         1.97325365692316183531D+01,-1.22097010558934838708D+01,
     3        -6.99732735041547247161D+00,-2.49999970104184464568D+00,
     4         7.49999999999027092188D-01/
C----------------------------------------------------------------------
      X = XX
      IF (ABS(X) .GT. XLARGE) THEN
            IF (ABS(X) .LE. XMAX) THEN
                  DDAW = HALF / X
               ELSE
                  DDAW = ZERO
            END IF
         ELSE IF (ABS(X) .LT. XSMALL) THEN
            DDAW = X
         ELSE
            Y = X * X
            IF (Y .LT. SIX25) THEN
C----------------------------------------------------------------------
C  ABS(X) .LT. 2.5 
C----------------------------------------------------------------------
                  SUMP = P1(1)
                  SUMQ = Q1(1)
                  DO 100 I = 2, 10
                     SUMP = SUMP * Y + P1(I)
                     SUMQ = SUMQ * Y + Q1(I)
  100             CONTINUE
                  DDAW = X * SUMP / SUMQ
               ELSE IF (Y .LT. ONE225) THEN
C----------------------------------------------------------------------
C  2.5 .LE. ABS(X) .LT. 3.5 
C----------------------------------------------------------------------
                  FRAC = ZERO
                  DO 200 I = 1, 9
  200                FRAC = Q2(I) / (P2(I) + Y + FRAC)
                  DDAW = (P2(10) + FRAC) / X
               ELSE IF (Y .LT. TWO5) THEN
C----------------------------------------------------------------------
C  3.5 .LE. ABS(X) .LT. 5.0 
C---------------------------------------------------------------------
                  FRAC = ZERO
                  DO 300 I = 1, 9
  300                FRAC = Q3(I) / (P3(I) + Y + FRAC)
                  DDAW = (P3(10) + FRAC) / X
               ELSE
C----------------------------------------------------------------------
C  5.0 .LE. ABS(X) .LE. XLARGE 
C------------------------------------------------------------------
                  W2 = ONE / X / X
                  FRAC = ZERO
                  DO 400 I = 1, 9
  400                FRAC = Q4(I) / (P4(I) + Y + FRAC)
                  FRAC = P4(10) + FRAC
                  DDAW = (HALF + HALF * W2 * FRAC) / X
            END IF
      END IF
      RETURN
C---------- Last line of DDAW ----------
      END