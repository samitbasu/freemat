      SUBROUTINE CGET54( N, A, LDA, B, LDB, S, LDS, T, LDT, U, LDU, V,
     $                   LDV, WORK, RESULT )
*
*  -- LAPACK test routine (version 3.0) --
*     Univ. of Tennessee, Univ. of California Berkeley, NAG Ltd.,
*     Courant Institute, Argonne National Lab, and Rice University
*     June 30, 1999
*
*     .. Scalar Arguments ..
      INTEGER            LDA, LDB, LDS, LDT, LDU, LDV, N
      REAL               RESULT
*     ..
*     .. Array Arguments ..
      COMPLEX            A( LDA, * ), B( LDB, * ), S( LDS, * ),
     $                   T( LDT, * ), U( LDU, * ), V( LDV, * ),
     $                   WORK( * )
*     ..
*
*  Purpose
*  =======
*
*  CGET54 checks a generalized decomposition of the form
*
*           A = U*S*V'  and B = U*T* V'
*
*  where ' means conjugate transpose and U and V are unitary.
*
*  Specifically,
*
*    RESULT = ||( A - U*S*V', B - U*T*V' )|| / (||( A, B )||*n*ulp )
*
*  Arguments
*  =========
*
*  N       (input) INTEGER
*          The size of the matrix.  If it is zero, SGET54 does nothing.
*          It must be at least zero.
*
*  A       (input) COMPLEX array, dimension (LDA, N)
*          The original (unfactored) matrix A.
*
*  LDA     (input) INTEGER
*          The leading dimension of A.  It must be at least 1
*          and at least N.
*
*  B       (input) COMPLEX array, dimension (LDB, N)
*          The original (unfactored) matrix B.
*
*  LDB     (input) INTEGER
*          The leading dimension of B.  It must be at least 1
*          and at least N.
*
*  S       (input) COMPLEX array, dimension (LDS, N)
*          The factored matrix S.
*
*  LDS     (input) INTEGER
*          The leading dimension of S.  It must be at least 1
*          and at least N.
*
*  T       (input) COMPLEX array, dimension (LDT, N)
*          The factored matrix T.
*
*  LDT     (input) INTEGER
*          The leading dimension of T.  It must be at least 1
*          and at least N.
*
*  U       (input) COMPLEX array, dimension (LDU, N)
*          The orthogonal matrix on the left-hand side in the
*          decomposition.
*
*  LDU     (input) INTEGER
*          The leading dimension of U.  LDU must be at least N and
*          at least 1.
*
*  V       (input) COMPLEX array, dimension (LDV, N)
*          The orthogonal matrix on the left-hand side in the
*          decomposition.
*
*  LDV     (input) INTEGER
*          The leading dimension of V.  LDV must be at least N and
*          at least 1.
*
*  WORK    (workspace) COMPLEX array, dimension (3*N**2)
*
*  RESULT  (output) REAL
*          The value RESULT, It is currently limited to 1/ulp, to
*          avoid overflow. Errors are flagged by RESULT=10/ulp.
*
*  =====================================================================
*
*     .. Parameters ..
      REAL               ZERO, ONE
      PARAMETER          ( ZERO = 0.0E+0, ONE = 1.0E+0 )
      COMPLEX            CZERO, CONE
      PARAMETER          ( CZERO = ( 0.0E+0, 0.0E+0 ),
     $                   CONE = ( 1.0E+0, 0.0E+0 ) )
*     ..
*     .. Local Scalars ..
      REAL               ABNORM, ULP, UNFL, WNORM
*     ..
*     .. Local Arrays ..
      REAL               DUM( 1 )
*     ..
*     .. External Functions ..
      REAL               CLANGE, SLAMCH
      EXTERNAL           CLANGE, SLAMCH
*     ..
*     .. External Subroutines ..
      EXTERNAL           CGEMM, CLACPY
*     ..
*     .. Intrinsic Functions ..
      INTRINSIC          MAX, MIN, REAL
*     ..
*     .. Executable Statements ..
*
      RESULT = ZERO
      IF( N.LE.0 )
     $   RETURN
*
*     Constants
*
      UNFL = SLAMCH( 'Safe minimum' )
      ULP = SLAMCH( 'Epsilon' )*SLAMCH( 'Base' )
*
*     compute the norm of (A,B)
*
      CALL CLACPY( 'Full', N, N, A, LDA, WORK, N )
      CALL CLACPY( 'Full', N, N, B, LDB, WORK( N*N+1 ), N )
      ABNORM = MAX( CLANGE( '1', N, 2*N, WORK, N, DUM ), UNFL )
*
*     Compute W1 = A - U*S*V', and put in the array WORK(1:N*N)
*
      CALL CLACPY( ' ', N, N, A, LDA, WORK, N )
      CALL CGEMM( 'N', 'N', N, N, N, CONE, U, LDU, S, LDS, CZERO,
     $            WORK( N*N+1 ), N )
*
      CALL CGEMM( 'N', 'C', N, N, N, -CONE, WORK( N*N+1 ), N, V, LDV,
     $            CONE, WORK, N )
*
*     Compute W2 = B - U*T*V', and put in the workarray W(N*N+1:2*N*N)
*
      CALL CLACPY( ' ', N, N, B, LDB, WORK( N*N+1 ), N )
      CALL CGEMM( 'N', 'N', N, N, N, CONE, U, LDU, T, LDT, CZERO,
     $            WORK( 2*N*N+1 ), N )
*
      CALL CGEMM( 'N', 'C', N, N, N, -CONE, WORK( 2*N*N+1 ), N, V, LDV,
     $            CONE, WORK( N*N+1 ), N )
*
*     Compute norm(W)/ ( ulp*norm((A,B)) )
*
      WNORM = CLANGE( '1', N, 2*N, WORK, N, DUM )
*
      IF( ABNORM.GT.WNORM ) THEN
         RESULT = ( WNORM / ABNORM ) / ( 2*N*ULP )
      ELSE
         IF( ABNORM.LT.ONE ) THEN
            RESULT = ( MIN( WNORM, 2*N*ABNORM ) / ABNORM ) / ( 2*N*ULP )
         ELSE
            RESULT = MIN( WNORM / ABNORM, REAL( 2*N ) ) / ( 2*N*ULP )
         END IF
      END IF
*
      RETURN
*
*     End of CGET54
*
      END
