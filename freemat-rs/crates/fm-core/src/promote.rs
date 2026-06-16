//! Type-promotion rules for binary operations.
//!
//! FreeMat's `ComputeTypes` (`Array.hpp`) is intentionally simple: arithmetic
//! happens in `double` unless both operands are `single` (then `single`), and
//! the result class follows the same rule. MATLAB additionally lets an integer
//! class combine with `double`/`single`/`logical`/`char` to yield that integer
//! class. We encode both: [`promote`] returns the class arithmetic should be
//! carried out in (and the result stored as).
//!
//! Rules (highest priority first):
//! 1. Two identical classes promote to themselves.
//! 2. An integer class combined with a *non-integer* numeric (`bool`, `char`,
//!    `single`, `double`) yields the integer class (MATLAB semantics).
//! 3. Mixing two *different* integer classes is an error (MATLAB forbids it).
//! 4. `single` with anything non-integer yields `single`.
//! 5. Otherwise `double`.

use crate::class::DataClass;
use crate::error::{CoreError, Result};

/// Compute the common arithmetic/result class for a binary numeric op.
///
/// # Errors
/// Returns [`CoreError::Incompatible`] when the classes cannot combine — e.g.
/// two distinct integer classes, or a reference (cell/struct) class.
pub fn promote(a: DataClass, b: DataClass) -> Result<DataClass> {
    use DataClass::{Double, Float};

    if a.is_reference() || b.is_reference() {
        return Err(CoreError::Incompatible { lhs: a, rhs: b });
    }

    if a == b {
        // char + char and bool + bool promote to double in MATLAB arithmetic.
        return Ok(match a {
            DataClass::Char | DataClass::Bool => Double,
            other => other,
        });
    }

    match (a.is_integer(), b.is_integer()) {
        // Two different integer classes: not allowed.
        (true, true) => Err(CoreError::Incompatible { lhs: a, rhs: b }),
        // Exactly one is an integer class: result is that integer class.
        (true, false) => Ok(a),
        (false, true) => Ok(b),
        // Neither is an integer: float lattice (single dominates only over
        // bool/char/itself; double wins otherwise).
        (false, false) => {
            if a == Float || b == Float {
                // single + (single|bool|char) => single; single + double => double.
                if a == Double || b == Double {
                    Ok(Double)
                } else {
                    Ok(Float)
                }
            } else {
                Ok(Double)
            }
        }
    }
}
