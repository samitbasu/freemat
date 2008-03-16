      SUBROUTINE ZGELSX( M, N, NRHS, A, LDA, B, LDB, JPVT, RCOND, RANK,
     $                   WORK, RWORK, INFO )
*
*  -- LAPACK driver routine (instrumented to count ops, version 3.0) --
*     Univ. of Tennessee, Univ. of California Berkeley, NAG Ltd.,
*     Courant Institute, Argonne National Lab, and Rice University
*     September 30, 1994
*
*     .. Scalar Arguments ..
      INTEGER            INFO, LDA, LDB, M, N, NRHS, RANK
      DOUBLE PRECISION   RCOND
*     ..
*     .. Array Arguments ..
      INTEGER            JPVT( * )
      DOUBLE PRECISION   RWORK( * )
      COMPLEX*16         A( LDA, * ), B( LDB, * ), WORK( * )
*     ..
*     Common blocks to return operation counts and timings
*     .. Common blocks ..
      COMMON             / LATIME / OPS, ITCNT
      COMMON             / LSTIME / OPCNT, TIMNG
*     ..
*     .. Scalars in Common ..
      DOUBLE PRECISION   ITCNT, OPS
*     ..
*     .. Arrays in Common ..
      DOUBLE PRECISION   OPCNT( 6 ), TIMNG( 6 )
*     ..
*
*  Purpose
*  =======
*
*  ZGELSX computes the minimum-norm solution to a complex linear least
*  squares problem:
*      minimize || A * X - B ||
*  using a complete orthogonal factorization of A.  A is an M-by-N
*  matrix which may be rank-deficient.
*
*  Several right hand side vectors b and solution vectors x can be
*  handled in a single call; they are stored as the columns of the
*  M-by-NRHS right hand side matrix B and the N-by-NRHS solution
*  matrix X.
*
*  The routine first computes a QR factorization with column pivoting:
*      A * P = Q * [ R11 R12 ]
*                  [  0  R22 ]
*  with R11 defined as the largest leading submatrix whose estimated
*  condition number is less than 1/RCOND.  The order of R11, RANK,
*  is the effective rank of A.
*
*  Then, R22 is considered to be negligible, and R12 is annihilated
*  by unitary transformations from the right, arriving at the
*  complete orthogonal factorization:
*     A * P = Q * [ T11 0 ] * Z
*                 [  0  0 ]
*  The minimum-norm solution is then
*     X = P * Z' [ inv(T11)*Q1'*B ]
*                [        0       ]
*  where Q1 consists of the first RANK columns of Q.
*
*  Arguments
*  =========
*
*  M       (input) INTEGER
*          The number of rows of the matrix A.  M >= 0.
*
*  N       (input) INTEGER
*          The number of columns of the matrix A.  N >= 0.
*
*  NRHS    (input) INTEGER
*          The number of right hand sides, i.e., the number of
*          columns of matrices B and X. NRHS >= 0.
*
*  A       (input/output) COMPLEX*16 array, dimension (LDA,N)
*          On entry, the M-by-N matrix A.
*          On exit, A has been overwritten by details of its
*          complete orthogonal factorization.
*
*  LDA     (input) INTEGER
*          The leading dimension of the array A.  LDA >= max(1,M).
*
*  B       (input/output) COMPLEX*16 array, dimension (LDB,NRHS)
*          On entry, the M-by-NRHS right hand side matrix B.
*          On exit, the N-by-NRHS solution matrix X.
*          If m >= n and RANK = n, the residual sum-of-squares for
*          the solution in the i-th column is given by the sum of
*          squares of elements N+1:M in that column.
*
*  LDB     (input) INTEGER
*          The leading dimension of the array B. LDB >= max(1,M,N).
*
*  JPVT    (input/output) INTEGER array, dimension (N)
*          On entry, if JPVT(i) .ne. 0, the i-th column of A is an
*          initial column, otherwise it is a free column.  Before
*          the QR factorization of A, all initial columns are
*          permuted to the leading positions; only the remaining
*          free columns are moved as a result of column pivoting
*          during the factorization.
*          On exit, if JPVT(i) = k, then the i-th column of A*P
*          was the k-th column of A.
*
*  RCOND   (input) DOUBLE PRECISION
*          RCOND is used to determine the effective rank of A, which
*          is defined as the order of the largest leading triangular
*          submatrix R11 in the QR factorization with pivoting of A,
*          whose estimated condition number < 1/RCOND.
*
*  RANK    (output) INTEGER
*          The effective rank of A, i.e., the order of the submatrix
*          R11.  This is the same as the order of the submatrix T11
*          in the complete orthogonal factorization of A.
*
*  WORK    (workspace) COMPLEX*16 array, dimension
*                      (min(M,N) + max( N, 2*min(M,N)+NRHS )),
*
*  RWORK   (workspace) DOUBLE PRECISION array, dimension (2*N)
*
*  INFO    (output) INTEGER
*          = 0:  successful exit
*          < 0:  if INFO = -i, the i-th argument had an illegal value
*
*  =====================================================================
*
*     .. Parameters ..
      INTEGER            IMAX, IMIN
      PARAMETER          ( IMAX = 1, IMIN = 2 )
      DOUBLE PRECISION   ZERO, ONE, DONE, NTDONE
      PARAMETER          ( ZERO = 0.0D0, ONE = 1.0D0, DONE = ZERO,
     $                   NTDONE = ONE )
      COMPLEX*16         CZERO, CONE
      PARAMETER          ( CZERO = ( 0.0D0, 0.0D0 ),
     $                   CONE = ( 1.0D0, 0.0D0 ) )
*     ..
*     .. Local Scalars ..
      INTEGER            GELSX, GEQPF, I, IASCL, IBSCL, ISMAX, ISMIN, J,
     $                   K, LATZM, MN, TRSM, TZRQF, UNM2R
      DOUBLE PRECISION   ANRM, BIGNUM, BNRM, SMAX, SMAXPR, SMIN, SMINPR,
     $                   SMLNUM, TIM1, TIM2
      COMPLEX*16         C1, C2, S1, S2, T1, T2
*     ..
*     .. External Functions ..
      DOUBLE PRECISION   DLAMCH, DOPBL3, DOPLA, DSECND, ZLANGE
      EXTERNAL           DLAMCH, DOPBL3, DOPLA, DSECND, ZLANGE
*     ..
*     .. External Subroutines ..
      EXTERNAL           DLABAD, XERBLA, ZGEQPF, ZLAIC1, ZLASCL, ZLASET,
     $                   ZLATZM, ZTRSM, ZTZRQF, ZUNM2R
*     ..
*     .. Intrinsic Functions ..
      INTRINSIC          ABS, DBLE, DCONJG, MAX, MIN
*     ..
*     .. Data statements ..
      DATA               GELSX / 1 / , GEQPF / 2 / , LATZM / 6 / ,
     $                   UNM2R / 4 / , TRSM / 5 / , TZRQF / 3 /
*     ..
*     .. Executable Statements ..
*
      MN = MIN( M, N )
      ISMIN = MN + 1
      ISMAX = 2*MN + 1
*
*     Test the input arguments.
*
      INFO = 0
      IF( M.LT.0 ) THEN
         INFO = -1
      ELSE IF( N.LT.0 ) THEN
         INFO = -2
      ELSE IF( NRHS.LT.0 ) THEN
         INFO = -3
      ELSE IF( LDA.LT.MAX( 1, M ) ) THEN
         INFO = -5
      ELSE IF( LDB.LT.MAX( 1, M, N ) ) THEN
         INFO = -7
      END IF
*
      IF( INFO.NE.0 ) THEN
         CALL XERBLA( 'ZGELSX', -INFO )
         RETURN
      END IF
*
*     Quick return if possible
*
      IF( MIN( M, N, NRHS ).EQ.0 ) THEN
         RANK = 0
         RETURN
      END IF
*
*     Get machine parameters
*
      OPCNT( GELSX ) = OPCNT( GELSX ) + DBLE( 2 )
      SMLNUM = DLAMCH( 'S' ) / DLAMCH( 'P' )
      BIGNUM = ONE / SMLNUM
      CALL DLABAD( SMLNUM, BIGNUM )
*
*     Scale A, B if max elements outside range [SMLNUM,BIGNUM]
*
      ANRM = ZLANGE( 'M', M, N, A, LDA, RWORK )
      IASCL = 0
      IF( ANRM.GT.ZERO .AND. ANRM.LT.SMLNUM ) THEN
*
*        Scale matrix norm up to SMLNUM
*
         OPCNT( GELSX ) = OPCNT( GELSX ) + DBLE( 6*M*N )
         CALL ZLASCL( 'G', 0, 0, ANRM, SMLNUM, M, N, A, LDA, INFO )
         IASCL = 1
      ELSE IF( ANRM.GT.BIGNUM ) THEN
*
*        Scale matrix norm down to BIGNUM
*
         OPCNT( GELSX ) = OPCNT( GELSX ) + DBLE( 6*M*N )
         CALL ZLASCL( 'G', 0, 0, ANRM, BIGNUM, M, N, A, LDA, INFO )
         IASCL = 2
      ELSE IF( ANRM.EQ.ZERO ) THEN
*
*        Matrix all zero. Return zero solution.
*
         CALL ZLASET( 'F', MAX( M, N ), NRHS, CZERO, CZERO, B, LDB )
         RANK = 0
         GO TO 100
      END IF
*
      BNRM = ZLANGE( 'M', M, NRHS, B, LDB, RWORK )
      IBSCL = 0
      IF( BNRM.GT.ZERO .AND. BNRM.LT.SMLNUM ) THEN
*
*        Scale matrix norm up to SMLNUM
*
         OPCNT( GELSX ) = OPCNT( GELSX ) + DBLE( 6*M*NRHS )
         CALL ZLASCL( 'G', 0, 0, BNRM, SMLNUM, M, NRHS, B, LDB, INFO )
         IBSCL = 1
      ELSE IF( BNRM.GT.BIGNUM ) THEN
*
*        Scale matrix norm down to BIGNUM
*
         OPCNT( GELSX ) = OPCNT( GELSX ) + DBLE( 6*M*NRHS )
         CALL ZLASCL( 'G', 0, 0, BNRM, BIGNUM, M, NRHS, B, LDB, INFO )
         IBSCL = 2
      END IF
*
*     Compute QR factorization with column pivoting of A:
*        A * P = Q * R
*
      OPCNT( GEQPF ) = OPCNT( GEQPF ) + DOPLA( 'ZGEQPF', M, N, 0, 0, 0 )
      TIM1 = DSECND( )
      CALL ZGEQPF( M, N, A, LDA, JPVT, WORK( 1 ), WORK( MN+1 ), RWORK,
     $             INFO )
      TIM2 = DSECND( )
      TIMNG( GEQPF ) = TIMNG( GEQPF ) + ( TIM2-TIM1 )
*
*     complex workspace MN+N. Real workspace 2*N. Details of Householder
*     rotations stored in WORK(1:MN).
*
*     Determine RANK using incremental condition estimation
*
      WORK( ISMIN ) = CONE
      WORK( ISMAX ) = CONE
      SMAX = ABS( A( 1, 1 ) )
      SMIN = SMAX
      IF( ABS( A( 1, 1 ) ).EQ.ZERO ) THEN
         RANK = 0
         CALL ZLASET( 'F', MAX( M, N ), NRHS, CZERO, CZERO, B, LDB )
         GO TO 100
      ELSE
         RANK = 1
      END IF
*
   10 CONTINUE
      IF( RANK.LT.MN ) THEN
         I = RANK + 1
         OPS = 0
         CALL ZLAIC1( IMIN, RANK, WORK( ISMIN ), SMIN, A( 1, I ),
     $                A( I, I ), SMINPR, S1, C1 )
         CALL ZLAIC1( IMAX, RANK, WORK( ISMAX ), SMAX, A( 1, I ),
     $                A( I, I ), SMAXPR, S2, C2 )
         OPCNT( GELSX ) = OPCNT( GELSX ) + OPS + DBLE( 1 )
*
         IF( SMAXPR*RCOND.LE.SMINPR ) THEN
            OPCNT( GELSX ) = OPCNT( GELSX ) + DBLE( RANK*6 )
            DO 20 I = 1, RANK
               WORK( ISMIN+I-1 ) = S1*WORK( ISMIN+I-1 )
               WORK( ISMAX+I-1 ) = S2*WORK( ISMAX+I-1 )
   20       CONTINUE
            WORK( ISMIN+RANK ) = C1
            WORK( ISMAX+RANK ) = C2
            SMIN = SMINPR
            SMAX = SMAXPR
            RANK = RANK + 1
            GO TO 10
         END IF
      END IF
*
*     Logically partition R = [ R11 R12 ]
*                             [  0  R22 ]
*     where R11 = R(1:RANK,1:RANK)
*
*     [R11,R12] = [ T11, 0 ] * Y
*
      IF( RANK.LT.N ) THEN
         OPCNT( TZRQF ) = OPCNT( TZRQF ) +
     $                    DOPLA( 'ZTZRQF', RANK, N, 0, 0, 0 )
         TIM1 = DSECND( )
         CALL ZTZRQF( RANK, N, A, LDA, WORK( MN+1 ), INFO )
         TIM2 = DSECND( )
         TIMNG( TZRQF ) = TIMNG( TZRQF ) + ( TIM2-TIM1 )
      END IF
*
*     Details of Householder rotations stored in WORK(MN+1:2*MN)
*
*     B(1:M,1:NRHS) := Q' * B(1:M,1:NRHS)
*
      OPCNT( UNM2R ) = OPCNT( UNM2R ) +
     $                 DOPLA( 'ZUNMQR', M, NRHS, MN, 0, 0 )
      TIM1 = DSECND( )
      CALL ZUNM2R( 'Left', 'Conjugate transpose', M, NRHS, MN, A, LDA,
     $             WORK( 1 ), B, LDB, WORK( 2*MN+1 ), INFO )
      TIM2 = DSECND( )
      TIMNG( UNM2R ) = TIMNG( UNM2R ) + ( TIM2-TIM1 )
*
*     workspace NRHS
*
*     B(1:RANK,1:NRHS) := inv(T11) * B(1:RANK,1:NRHS)
*
      OPCNT( TRSM ) = OPCNT( TRSM ) + DOPBL3( 'ZTRSM ', RANK, NRHS, 0 )
      TIM1 = DSECND( )
      CALL ZTRSM( 'Left', 'Upper', 'No transpose', 'Non-unit', RANK,
     $            NRHS, CONE, A, LDA, B, LDB )
      TIM2 = DSECND( )
      TIMNG( TRSM ) = TIMNG( TRSM ) + ( TIM2-TIM1 )
*
      DO 40 I = RANK + 1, N
         DO 30 J = 1, NRHS
            B( I, J ) = CZERO
   30    CONTINUE
   40 CONTINUE
*
*     B(1:N,1:NRHS) := Y' * B(1:N,1:NRHS)
*
      IF( RANK.LT.N ) THEN
         OPCNT( LATZM ) = OPCNT( LATZM ) +
     $                    DBLE( 8*( ( N-RANK )*NRHS+NRHS+( N-RANK )*
     $                    NRHS )*RANK )
         TIM1 = DSECND( )
         DO 50 I = 1, RANK
            CALL ZLATZM( 'Left', N-RANK+1, NRHS, A( I, RANK+1 ), LDA,
     $                   DCONJG( WORK( MN+I ) ), B( I, 1 ),
     $                   B( RANK+1, 1 ), LDB, WORK( 2*MN+1 ) )
   50    CONTINUE
         TIM2 = DSECND( )
         TIMNG( LATZM ) = TIMNG( LATZM ) + ( TIM2-TIM1 )
      END IF
*
*     workspace NRHS
*
*     B(1:N,1:NRHS) := P * B(1:N,1:NRHS)
*
      DO 90 J = 1, NRHS
         DO 60 I = 1, N
            WORK( 2*MN+I ) = NTDONE
   60    CONTINUE
         DO 80 I = 1, N
            IF( WORK( 2*MN+I ).EQ.NTDONE ) THEN
               IF( JPVT( I ).NE.I ) THEN
                  K = I
                  T1 = B( K, J )
                  T2 = B( JPVT( K ), J )
   70             CONTINUE
                  B( JPVT( K ), J ) = T1
                  WORK( 2*MN+K ) = DONE
                  T1 = T2
                  K = JPVT( K )
                  T2 = B( JPVT( K ), J )
                  IF( JPVT( K ).NE.I )
     $               GO TO 70
                  B( I, J ) = T1
                  WORK( 2*MN+K ) = DONE
               END IF
            END IF
   80    CONTINUE
   90 CONTINUE
*
*     Undo scaling
*
      IF( IASCL.EQ.1 ) THEN
         OPCNT( GELSX ) = OPCNT( GELSX ) +
     $                    DBLE( 6*( N*NRHS+RANK*RANK ) )
         CALL ZLASCL( 'G', 0, 0, ANRM, SMLNUM, N, NRHS, B, LDB, INFO )
         CALL ZLASCL( 'U', 0, 0, SMLNUM, ANRM, RANK, RANK, A, LDA,
     $                INFO )
      ELSE IF( IASCL.EQ.2 ) THEN
         OPCNT( GELSX ) = OPCNT( GELSX ) +
     $                    DBLE( 6*( N*NRHS+RANK*RANK ) )
         CALL ZLASCL( 'G', 0, 0, ANRM, BIGNUM, N, NRHS, B, LDB, INFO )
         CALL ZLASCL( 'U', 0, 0, BIGNUM, ANRM, RANK, RANK, A, LDA,
     $                INFO )
      END IF
      IF( IBSCL.EQ.1 ) THEN
         OPCNT( GELSX ) = OPCNT( GELSX ) + DBLE( 6*N*NRHS )
         CALL ZLASCL( 'G', 0, 0, SMLNUM, BNRM, N, NRHS, B, LDB, INFO )
      ELSE IF( IBSCL.EQ.2 ) THEN
         OPCNT( GELSX ) = OPCNT( GELSX ) + DBLE( 6*N*NRHS )
         CALL ZLASCL( 'G', 0, 0, BIGNUM, BNRM, N, NRHS, B, LDB, INFO )
      END IF
*
  100 CONTINUE
*
      RETURN
*
*     End of ZGELSX
*
      END