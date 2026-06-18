//! The tree-walking interpreter: expression evaluation, statement execution,
//! control flow, assignment (including LHS indexing and multi-return), and
//! function calls.
//!
//! # Debug-readiness seams (Stage 3, used by Stage 10)
//! - [`Interpreter::exec_statement`] is the **single statement chokepoint**;
//!   every statement routes through it, so a breakpoint check slots in here.
//! - It threads the executing statement's **source span/line into the active
//!   scope** ([`Context::set_current_span`]) before running it.
//! - The [`Context`]'s **active scope is switchable** ([`Context::set_active`]),
//!   the basis for `dbup`/`dbdown`.

use std::collections::HashMap;
use std::sync::Arc;

use fm_core::{Array, C64, DataClass, Dims, FormatMode, ScalarValue};
use fm_parser::ast::{
    BinaryOp, Case, Expr, ExprKind, FunctionDef, Program, Stmt, StmtKind, Terminator,
};
use fm_parser::{Span, parse_program};
use smallvec::SmallVec;

use crate::context::Context;
use crate::error::{Flow, InterpError, Signal};
use crate::function::{Function, FunctionTable};
use crate::graphics::GraphicsState;
use crate::index::{self, IndexArg, IndexPlan};
use crate::ops;
use crate::value::{self, truth};

/// The tree-walking interpreter — owns the [`Context`] and function table.
pub struct Interpreter {
    /// The execution context (scopes, globals, persistents, call stack).
    pub context: Context,
    /// The function table (builtins + loaded `.m` functions).
    pub functions: FunctionTable,
    /// The display format mode (`format short` / `long`).
    pub format: FormatMode,
    /// Buffered output (so tests can assert on `disp`/echo output).
    output: String,
    /// `nargin` for the executing function frame (parallel to the scope stack).
    nargin_stack: Vec<usize>,
    /// `nargout` for the executing function frame.
    nargout_stack: Vec<usize>,
    /// Retained graphics scene + current figure + optional broadcast sink.
    pub graphics: GraphicsState,
    /// Stack of file-local function tables (parallel to the call stack). The
    /// top frame's locals shadow the global function table, giving FreeMat's
    /// file-private subfunction scoping.
    local_funcs_stack: Vec<Arc<crate::function::FileLocals>>,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    /// A fresh interpreter with the default builtins registered.
    #[must_use]
    pub fn new() -> Self {
        let mut interp = Interpreter {
            context: Context::new(),
            functions: FunctionTable::new(),
            format: FormatMode::Short,
            output: String::new(),
            nargin_stack: vec![0],
            nargout_stack: vec![0],
            graphics: GraphicsState::default(),
            local_funcs_stack: Vec::new(),
        };
        crate::builtins::register_defaults(&mut interp.functions);
        interp
    }

    /// Install a graphics sink (the webserver) so plotting commands broadcast
    /// scene updates. Without one, graphics builtins still update the retained
    /// scene but don't publish.
    pub fn set_graphics_sink(&mut self, sink: Box<dyn fm_graphics::GraphicsSink>) {
        self.graphics.sink = Some(sink);
    }

    /// Take the accumulated output (echoed values + `disp`), clearing it.
    pub fn take_output(&mut self) -> String {
        std::mem::take(&mut self.output)
    }

    /// Append to the output buffer (used by builtins like `disp`).
    pub fn emit(&mut self, s: &str) {
        self.output.push_str(s);
    }

    // ---- Top-level entry points --------------------------------------------

    /// Parse and run a source unit (`.m` script or REPL line). Function files
    /// are loaded into the function table; scripts execute their statements.
    ///
    /// # Errors
    /// Returns the runtime [`InterpError`] if execution raises one.
    pub fn run(&mut self, src: &str) -> Result<(), InterpError> {
        let program = parse_program(src).map_err(|e| InterpError::msg(e.to_string()))?;
        let result = match program {
            Program::Script(stmts) => self.run_block(&stmts, src).map_err(unwrap_error),
            Program::Functions(defs) => {
                self.load_functions(defs, src);
                Ok(())
            }
        };
        // Implicit draw: after a top-level unit, flush any scene changes through
        // the sink (mirrors MATLAB drawing the figure once the command returns;
        // a trailing `;` suppresses value echo, NOT the plot).
        if self.graphics.dirty {
            self.graphics.flush();
        }
        result
    }

    /// Run a list of statements (a script / REPL body) at the top level.
    pub fn run_block(&mut self, stmts: &[Stmt], src: &str) -> Flow<()> {
        for stmt in stmts {
            match self.exec_statement(stmt, src) {
                Ok(()) => {}
                // Break/Continue/Return at top level just stop the block.
                Err(Signal::Break | Signal::Continue | Signal::Return) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Register interpreted function definitions in the function table.
    pub fn load_functions(&mut self, defs: Vec<FunctionDef>, src: &str) {
        let shared = Arc::new(src.to_string());

        // A `.m` function file contains the public/main function (the first
        // top-level `function`) plus any subfunctions (subsequent top-level
        // functions) and nested functions. FreeMat scopes subfunctions and
        // nested functions to the *file*: they are callable only from other
        // functions in the same file, and must NOT clobber identically named
        // functions from other files (e.g. a test file's private `testeq`).
        //
        // We collect every function defined in the file into a shared
        // file-local table, register only the main function in the global table
        // (with that table attached), and resolve file-locals first at call
        // time (see `local_funcs_stack` / `call_function`).
        let mut funcs: crate::function::FileLocals = crate::function::FileLocals {
            funcs: HashMap::new(),
            src: shared,
        };
        let mut main: Option<Arc<FunctionDef>> = None;
        for def in defs {
            for n in &def.nested {
                let n = Arc::new(n.clone());
                funcs.funcs.entry(n.name.clone()).or_insert(n);
            }
            let def = Arc::new(def);
            // The first top-level function is the public one (matches the
            // filename in FreeMat); the rest are file-private subfunctions.
            if main.is_none() {
                main = Some(def.clone());
            }
            funcs.funcs.entry(def.name.clone()).or_insert(def);
        }
        let locals = Arc::new(funcs);

        // Only the public function is registered globally; subfunctions stay
        // private to the file via `locals`.
        if let Some(main) = main {
            self.functions.add_interpreted(main, locals);
        }
    }

    // ---- The statement chokepoint (DEBUG SEAM) ------------------------------

    /// Execute one statement. **This is the single execution chokepoint**: every
    /// statement flows through here, and the executing statement's span/line is
    /// threaded into the active scope before running. A future debugger checks
    /// breakpoints right here (Stage 10) — no retrofit needed.
    pub fn exec_statement(&mut self, stmt: &Stmt, src: &str) -> Flow<()> {
        // DEBUG SEAM: record the current location in the top scope.
        let line = line_of(src, stmt.span.start);
        self.context.set_current_span(stmt.span, line);
        // (Stage 10 inserts: self.check_breakpoint(stmt)?; here.)

        self.exec_statement_inner(stmt, src)
    }

    fn exec_statement_inner(&mut self, stmt: &Stmt, src: &str) -> Flow<()> {
        match &stmt.kind {
            StmtKind::Expr(e) => self.exec_expr_stmt(e, stmt.terminator, src),
            StmtKind::Assign { lhs, rhs } => self.exec_assign(lhs, rhs, stmt.terminator, src),
            StmtKind::MultiAssign { lhs, rhs } => {
                self.exec_multi_assign(lhs, rhs, stmt.terminator, src)
            }
            StmtKind::Command { name, args } => self.exec_command(name, args, src, stmt.span),
            StmtKind::For { var, range, body } => self.exec_for(var, range, body, src),
            StmtKind::While { cond, body } => self.exec_while(cond, body, src),
            StmtKind::If {
                cond,
                body,
                elseifs,
                else_body,
            } => self.exec_if(cond, body, elseifs, else_body.as_deref(), src),
            StmtKind::Switch {
                scrutinee,
                cases,
                otherwise,
            } => self.exec_switch(scrutinee, cases, otherwise.as_deref(), src),
            StmtKind::Try { body, catch } => self.exec_try(body, catch.as_deref(), src),
            StmtKind::Global(names) => {
                for n in names {
                    self.context.declare_global(n);
                }
                Ok(())
            }
            StmtKind::Persistent(names) => {
                for n in names {
                    self.context.declare_persistent(n);
                }
                Ok(())
            }
            StmtKind::Keyword(kw) => self.exec_keyword(*kw),
            StmtKind::FunctionDef(def) => {
                self.load_functions(vec![(**def).clone()], src);
                Ok(())
            }
        }
    }

    fn exec_block(&mut self, stmts: &[Stmt], src: &str) -> Flow<()> {
        for stmt in stmts {
            self.exec_statement(stmt, src)?;
        }
        Ok(())
    }

    // ---- Statement kinds ----------------------------------------------------

    fn exec_keyword(&mut self, kw: fm_parser::ast::KeywordStmt) -> Flow<()> {
        use fm_parser::ast::KeywordStmt;
        match kw {
            KeywordStmt::Break => Err(Signal::Break),
            KeywordStmt::Continue => Err(Signal::Continue),
            KeywordStmt::Return | KeywordStmt::Retall => Err(Signal::Return),
            // Debugger keywords: no-ops in Stage 3 (engine lands in Stage 10).
            KeywordStmt::Keyboard | KeywordStmt::DbUp | KeywordStmt::DbDown => Ok(()),
            KeywordStmt::Quit => Err(Signal::Return),
        }
    }

    fn exec_expr_stmt(&mut self, e: &Expr, term: Terminator, src: &str) -> Flow<()> {
        // A bare identifier / call may legitimately produce zero outputs
        // (a command), so evaluate for up to one output but tolerate none.
        let values = self.eval_multi(e, 1, src)?;
        if let Some(v) = values.into_iter().next() {
            // Assign to `ans` and echo unless suppressed.
            self.context.assign("ans", v.clone());
            if term == Terminator::Display {
                self.echo("ans", &v);
            }
        }
        Ok(())
    }

    fn exec_assign(&mut self, lhs: &Expr, rhs: &Expr, term: Terminator, src: &str) -> Flow<()> {
        let value = self.eval(rhs, src)?;
        let name = self.assign_to(lhs, value, src)?;
        if term == Terminator::Display
            && let Some(v) = self.context.lookup(&name)
        {
            let v = v.clone();
            self.echo(&name, &v);
        }
        Ok(())
    }

    fn exec_multi_assign(
        &mut self,
        lhs: &[Expr],
        rhs: &Expr,
        term: Terminator,
        src: &str,
    ) -> Flow<()> {
        // Each LHS target normally consumes one returned value, but a
        // cell-content target with a multi-element subscript — `[c{1:3}] = f` —
        // consumes one value *per* indexed position (MATLAB/FreeMat comma-list
        // expansion). Compute how many outputs each target wants so the total
        // `nargout` and the per-target distribution are correct.
        let mut slots: Vec<usize> = Vec::with_capacity(lhs.len());
        for target in lhs {
            slots.push(self.lhs_slot_count(target, src)?);
        }
        let nargout: usize = slots.iter().sum();
        let values = self.eval_multi(rhs, nargout, src)?;
        // FreeMat assigns LHS targets only while returned values remain: a call
        // that yields fewer values than requested (e.g. `[c,d] = size(a,2)`,
        // which returns one value) leaves the trailing targets unassigned rather
        // than erroring. We still error if the *first* target can't be filled.
        if values.is_empty() && !lhs.is_empty() {
            return Err(Signal::Error(InterpError::msg(
                "expression produced no value where one was required",
            )));
        }
        let mut names = Vec::with_capacity(lhs.len());
        let mut vit = values.into_iter().peekable();
        for (target, &count) in lhs.iter().zip(&slots) {
            if vit.peek().is_none() {
                break;
            }
            // `~` placeholder (parsed as Ident "~") discards the output.
            if matches!(&target.kind, ExprKind::Ident(n) if n == "~") {
                vit.next();
                continue;
            }
            if count == 1 {
                let value = vit.next().expect("peeked Some above");
                names.push(self.assign_to(target, value, src)?);
            } else {
                // Distribute up to `count` values across the cell-content
                // positions (taking fewer if the call returned fewer).
                let chunk: Vec<Array> = (0..count).map_while(|_| vit.next()).collect();
                names.push(self.assign_to_multi(target, chunk, src)?);
            }
        }
        if term == Terminator::Display {
            for name in names {
                if let Some(v) = self.context.lookup(&name) {
                    let v = v.clone();
                    self.echo(&name, &v);
                }
            }
        }
        Ok(())
    }

    fn exec_command(&mut self, name: &str, args: &[String], src: &str, span: Span) -> Flow<()> {
        let arg_vals: Vec<Array> = args.iter().map(|a| Array::char_string(a)).collect();
        self.call_function(name, &arg_vals, 0, src, span)?;
        Ok(())
    }

    fn exec_for(&mut self, var: &str, range: &Expr, body: &[Stmt], src: &str) -> Flow<()> {
        let values = self.eval(range, src)?;
        // Iterate over the columns of the range value (MATLAB `for` semantics).
        let cols = columns_of(&values);
        for col in cols {
            self.context.assign(var, col);
            match self.exec_block(body, src) {
                Ok(()) => {}
                Err(Signal::Break) => break,
                Err(Signal::Continue) => continue,
                Err(other) => return Err(other),
            }
        }
        Ok(())
    }

    fn exec_while(&mut self, cond: &Expr, body: &[Stmt], src: &str) -> Flow<()> {
        loop {
            let c = self.eval(cond, src)?;
            if !truth(&c)? {
                break;
            }
            match self.exec_block(body, src) {
                Ok(()) => {}
                Err(Signal::Break) => break,
                Err(Signal::Continue) => continue,
                Err(other) => return Err(other),
            }
        }
        Ok(())
    }

    fn exec_if(
        &mut self,
        cond: &Expr,
        body: &[Stmt],
        elseifs: &[fm_parser::ast::ElseIf],
        else_body: Option<&[Stmt]>,
        src: &str,
    ) -> Flow<()> {
        let c = self.eval(cond, src)?;
        if truth(&c)? {
            return self.exec_block(body, src);
        }
        for arm in elseifs {
            let c = self.eval(&arm.cond, src)?;
            if truth(&c)? {
                return self.exec_block(&arm.body, src);
            }
        }
        if let Some(eb) = else_body {
            return self.exec_block(eb, src);
        }
        Ok(())
    }

    fn exec_switch(
        &mut self,
        scrutinee: &Expr,
        cases: &[Case],
        otherwise: Option<&[Stmt]>,
        src: &str,
    ) -> Flow<()> {
        let value = self.eval(scrutinee, src)?;
        // FreeMat: the switch argument must be a scalar or a string (a vector or
        // matrix is an error — `test_switch4`).
        let is_scalar = value.numel() == 1;
        if !is_scalar && !value.is_char() {
            return Err(Signal::Error(InterpError::msg(
                "Switch statements support scalar and string arguments only.",
            )));
        }
        for case in cases {
            let label = self.eval(&case.label, src)?;
            if switch_matches(&value, &label) {
                return self.exec_block(&case.body, src);
            }
        }
        if let Some(ob) = otherwise {
            return self.exec_block(ob, src);
        }
        Ok(())
    }

    fn exec_try(&mut self, body: &[Stmt], catch: Option<&[Stmt]>, src: &str) -> Flow<()> {
        match self.exec_block(body, src) {
            Ok(()) => Ok(()),
            Err(Signal::Error(e)) => {
                // Bind the error message to `lasterr`-style; FreeMat binds the
                // catch identifier to an MException. We expose the message.
                self.context
                    .assign("lasterr", Array::char_string(&e.message));
                if let Some(handler) = catch {
                    self.exec_block(handler, src)
                } else {
                    Ok(())
                }
            }
            // Break/Continue/Return propagate out of try just like normal flow.
            Err(other) => Err(other),
        }
    }

    // ---- Assignment target resolution (LHS indexing, grow-on-assign) --------

    /// Assign `value` to the (possibly indexed) `target`, returning the root
    /// variable name affected (for echo).
    /// How many returned values a multi-assign LHS target consumes. Normally 1,
    /// but a cell-content index `c{i:j}` with a multi-element subscript consumes
    /// one value per indexed position (comma-list expansion).
    fn lhs_slot_count(&mut self, target: &Expr, src: &str) -> Flow<usize> {
        if let ExprKind::CellIndex { base, args } = &target.kind {
            let (_, current) = self.lvalue_base(base, src)?;
            let plan = self.plan_for(&current, args, src)?;
            return Ok(plan.linear.len().max(1));
        }
        Ok(1)
    }

    /// Assign several values into a multi-position cell-content target
    /// (`[c{1:3}] = f`): each indexed cell slot receives one of `values`.
    fn assign_to_multi(&mut self, target: &Expr, values: Vec<Array>, src: &str) -> Flow<String> {
        let ExprKind::CellIndex { base, args } = &target.kind else {
            // Only cell-content targets expand; fall back to single-value assign.
            let mut it = values.into_iter();
            let first = it.next().unwrap_or_else(Array::empty);
            return self.assign_to(target, first, src);
        };
        let (name, current) = self.lvalue_base(base, src)?;
        let plan = self.plan_for(&current, args, src)?;
        let updated = index::scatter_cell_contents_multi(&current, &plan, values)?;
        self.store_lvalue(base, updated, src)?;
        Ok(name)
    }

    fn assign_to(&mut self, target: &Expr, value: Array, src: &str) -> Flow<String> {
        match &target.kind {
            ExprKind::Ident(name) => {
                self.context.assign(name, value);
                Ok(name.clone())
            }
            ExprKind::Index { base, args } => {
                // Fast path for `A(idx) = value` where the base is a plain
                // variable: **take** the array out of the symbol table (so its
                // backing `Arc` is unshared when not aliased), build the plan,
                // scatter **in place** (`make_mut_*` deep-copies only on share —
                // preserving copy-on-write), then **put it back**. This avoids
                // the O(N) whole-array materialise+rebuild per element write.
                if let ExprKind::Ident(name) = &base.kind {
                    // Build the plan FIRST, while the variable is still bound:
                    // the index expression may reference the variable itself
                    // (e.g. `v(v > 2) = 0`, `A(end) = ...`). It needs `&mut self`
                    // to evaluate `i`,`j`, so it must complete *before* the slot
                    // is borrowed. Read the target's dims without cloning data
                    // (`shape()`). Missing variable → grow-from-nothing: use the
                    // empty array's `[0, 0]` dims so the plan grows identically.
                    let dims: Dims = self
                        .context
                        .lookup(name)
                        .map_or_else(|| SmallVec::from_slice(&[0, 0]), Array::dims_smallvec);
                    let plan = self.plan_for_dims(&dims, args, Some(&value.dims()), src)?;
                    // Now borrow the slot MUTABLY and scatter IN PLACE — no map
                    // remove/insert, no key realloc. COW stays correct:
                    // `scatter_into` → `make_mut_*` deep-copies only when the
                    // backing `Arc` is shared (`B = A`). Growth/retype reassigns
                    // `*slot` through the `&mut Array`. A brand-new variable
                    // (`get_mut` → None) falls back to insert-via-`set`.
                    if let Some(slot) = self.context.get_mut(name) {
                        index::scatter_into(slot, &plan, &value)?;
                    } else {
                        let mut current = Array::empty();
                        index::scatter_into(&mut current, &plan, &value)?;
                        self.context.set(name, current);
                    }
                    return Ok(name.clone());
                }
                // Nested l-value (`a.b(2)`, `a(1).b(3)`): keep the existing
                // evaluate-base → scatter → store-back path.
                let (name, current) = self.lvalue_base(base, src)?;
                let cur_dims = current.dims();
                let plan = self.plan_for_dims(&cur_dims, args, Some(&value.dims()), src)?;
                let updated = index::scatter(&current, &plan, &value)?;
                self.store_lvalue(base, updated, src)?;
                Ok(name)
            }
            ExprKind::CellIndex { base, args } => {
                let (name, current) = self.lvalue_base(base, src)?;
                let plan = self.plan_for(&current, args, src)?;
                let updated = index::scatter_cell_contents(&current, &plan, value)?;
                self.store_lvalue(base, updated, src)?;
                Ok(name)
            }
            ExprKind::Field { base, name: field } => {
                let (name, current) = self.lvalue_base(base, src)?;
                let updated = index::field_write(&current, field, value)?;
                self.store_lvalue(base, updated, src)?;
                Ok(name)
            }
            ExprKind::DynField { base, name } => {
                let fname = self.eval(name, src)?;
                let field = fname.as_string().ok_or_else(|| {
                    Signal::Error(InterpError::msg("dynamic field name must be a string"))
                })?;
                let (root, current) = self.lvalue_base(base, src)?;
                let updated = index::field_write(&current, &field, value)?;
                self.store_lvalue(base, updated, src)?;
                Ok(root)
            }
            _ => Err(Signal::Error(
                InterpError::msg("invalid left-hand side of assignment").located(src, target.span),
            )),
        }
    }

    /// Resolve the current value of an l-value base, defaulting to `[]` when the
    /// variable does not yet exist (grow-from-nothing).
    fn lvalue_base(&mut self, base: &Expr, src: &str) -> Flow<(String, Array)> {
        match &base.kind {
            ExprKind::Ident(name) => {
                let current = self
                    .context
                    .lookup(name)
                    .cloned()
                    .unwrap_or_else(Array::empty);
                Ok((name.clone(), current))
            }
            // Nested l-value (e.g. `a.b(2)` or `a(1).b`): evaluate the base for
            // its current value and remember its root name.
            ExprKind::Index { .. }
            | ExprKind::CellIndex { .. }
            | ExprKind::Field { .. }
            | ExprKind::DynField { .. } => {
                let root = root_name(base).ok_or_else(|| {
                    Signal::Error(InterpError::msg("unsupported assignment target"))
                })?;
                let current = self.eval(base, src).unwrap_or_else(|_| Array::empty());
                Ok((root, current))
            }
            _ => Err(Signal::Error(InterpError::msg("invalid assignment target"))),
        }
    }

    /// Store an updated value back through an l-value base (recursing for nested
    /// targets like `a.b(2) = x`).
    fn store_lvalue(&mut self, base: &Expr, value: Array, src: &str) -> Flow<()> {
        match &base.kind {
            ExprKind::Ident(name) => {
                self.context.assign(name, value);
                Ok(())
            }
            _ => {
                self.assign_to(base, value, src)?;
                Ok(())
            }
        }
    }

    // ---- Expression evaluation ----------------------------------------------

    /// Evaluate an expression to a single value.
    ///
    /// The single-value expression kinds are handled **directly here**, returning
    /// the `Array` by value with no intermediate `vec![..]` — this is the hot
    /// path (every operand of every arithmetic op, every index subscript). Only
    /// the genuinely multi-valued kinds (a bare identifier that may be a
    /// zero-arg function call, paren-indexing that may be a function call, and
    /// brace cell-content indexing) delegate to [`eval_multi`](Self::eval_multi)
    /// and take its first result.
    pub fn eval(&mut self, e: &Expr, src: &str) -> Flow<Array> {
        match &e.kind {
            ExprKind::Real { text, single } => real_literal(text, *single),
            ExprKind::Imag { text, single } => imag_literal(text, *single),
            ExprKind::Str(s) => Ok(Array::char_string(s)),
            ExprKind::End => Err(Signal::Error(InterpError::msg(
                "'end' used outside of an index context",
            ))),
            ExprKind::Colon => Err(Signal::Error(InterpError::msg(
                "':' used outside of an index context",
            ))),
            ExprKind::Unary { op, operand } => {
                let v = self.eval(operand, src)?;
                ops::unary(*op, &v)
            }
            ExprKind::Binary { op, lhs, rhs } => self.eval_binary(*op, lhs, rhs, src),
            ExprKind::Range { start, step, stop } => {
                self.eval_range(start, step.as_deref(), stop, src)
            }
            ExprKind::Transpose { operand, conjugate } => {
                let v = self.eval(operand, src)?;
                transpose(&v, *conjugate)
            }
            ExprKind::Field { base, name } => {
                let b = self.eval(base, src)?;
                index::field_read(&b, name)
            }
            ExprKind::DynField { base, name } => {
                let b = self.eval(base, src)?;
                let fname = self.eval(name, src)?;
                let field = fname.as_string().ok_or_else(|| {
                    Signal::Error(InterpError::msg("dynamic field name must be a string"))
                })?;
                index::field_read(&b, &field)
            }
            ExprKind::Matrix(rows) => self.eval_matrix(rows, src),
            ExprKind::Cell(rows) => self.eval_cell(rows, src),
            ExprKind::FuncHandle(_) | ExprKind::AnonFunc { .. } => Err(Signal::Error(
                InterpError::msg("function handles are not yet implemented (Stage 5/6)")
                    .located(src, e.span),
            )),
            // Possibly multi-valued (function call / cell-content): take first.
            ExprKind::Ident(_) | ExprKind::Index { .. } | ExprKind::CellIndex { .. } => {
                let vals = self.eval_multi(e, 1, src)?;
                vals.into_iter().next().ok_or_else(|| {
                    Signal::Error(
                        InterpError::msg("expression produced no value where one was required")
                            .located(src, e.span),
                    )
                })
            }
        }
    }

    /// Evaluate an expression that may yield multiple values (function calls,
    /// cell-content indexing). `nargout` requests that many outputs.
    pub fn eval_multi(&mut self, e: &Expr, nargout: usize, src: &str) -> Flow<Vec<Array>> {
        match &e.kind {
            ExprKind::Real { text, single } => Ok(vec![real_literal(text, *single)?]),
            ExprKind::Imag { text, single } => Ok(vec![imag_literal(text, *single)?]),
            ExprKind::Str(s) => Ok(vec![Array::char_string(s)]),
            ExprKind::End => Err(Signal::Error(InterpError::msg(
                "'end' used outside of an index context",
            ))),
            ExprKind::Colon => Err(Signal::Error(InterpError::msg(
                "':' used outside of an index context",
            ))),
            ExprKind::Ident(name) => self.eval_ident(name, nargout, src, e.span),
            ExprKind::Unary { op, operand } => {
                let v = self.eval(operand, src)?;
                Ok(vec![ops::unary(*op, &v)?])
            }
            ExprKind::Binary { op, lhs, rhs } => Ok(vec![self.eval_binary(*op, lhs, rhs, src)?]),
            ExprKind::Range { start, step, stop } => {
                Ok(vec![self.eval_range(start, step.as_deref(), stop, src)?])
            }
            ExprKind::Transpose { operand, conjugate } => {
                let v = self.eval(operand, src)?;
                Ok(vec![transpose(&v, *conjugate)?])
            }
            ExprKind::Index { base, args } => self.eval_index(base, args, nargout, src, e.span),
            ExprKind::CellIndex { base, args } => self.eval_cell_index(base, args, src),
            ExprKind::Field { base, name } => {
                let b = self.eval(base, src)?;
                Ok(vec![index::field_read(&b, name)?])
            }
            ExprKind::DynField { base, name } => {
                let b = self.eval(base, src)?;
                let fname = self.eval(name, src)?;
                let field = fname.as_string().ok_or_else(|| {
                    Signal::Error(InterpError::msg("dynamic field name must be a string"))
                })?;
                Ok(vec![index::field_read(&b, &field)?])
            }
            ExprKind::Matrix(rows) => Ok(vec![self.eval_matrix(rows, src)?]),
            ExprKind::Cell(rows) => Ok(vec![self.eval_cell(rows, src)?]),
            ExprKind::FuncHandle(_) | ExprKind::AnonFunc { .. } => Err(Signal::Error(
                InterpError::msg("function handles are not yet implemented (Stage 5/6)")
                    .located(src, e.span),
            )),
        }
    }

    fn eval_ident(
        &mut self,
        name: &str,
        nargout: usize,
        src: &str,
        span: Span,
    ) -> Flow<Vec<Array>> {
        // A bound variable always shadows a same-named builtin constant or
        // function (MATLAB/FreeMat treat `pi`, `e`, `eps`, `i`, … as *functions*,
        // so a local of that name wins). Check the scope first.
        if let Some(v) = self.context.lookup(name) {
            return Ok(vec![v.clone()]);
        }
        // Special interpreter constants / pseudo-variables (only when not a
        // user variable, per the lookup above).
        match name {
            "nargin" => return Ok(vec![Array::double(self.current_nargin() as f64)]),
            "nargout" => return Ok(vec![Array::double(self.current_nargout() as f64)]),
            "pi" => return Ok(vec![Array::double(std::f64::consts::PI)]),
            "e" => return Ok(vec![Array::double(std::f64::consts::E)]),
            "Inf" | "inf" => return Ok(vec![Array::double(f64::INFINITY)]),
            "NaN" | "nan" => return Ok(vec![Array::double(f64::NAN)]),
            "eps" => return Ok(vec![Array::double(f64::EPSILON)]),
            "i" | "j" | "I" | "J" => {
                return Ok(vec![Array::complex64(C64::new(0.0, 1.0))]);
            }
            "true" => {
                return Ok(vec![Array::bool(true)]);
            }
            "false" => {
                return Ok(vec![Array::bool(false)]);
            }
            _ => {}
        }
        // Not a variable → a zero-argument function call.
        if self.resolves_function(name) {
            return self.call_function(name, &[], nargout, src, span);
        }
        Err(Signal::Error(
            InterpError::msg(format!("undefined variable or function '{name}'")).located(src, span),
        ))
    }

    fn eval_binary(&mut self, op: BinaryOp, lhs: &Expr, rhs: &Expr, src: &str) -> Flow<Array> {
        // Short-circuit operators evaluate the rhs lazily.
        match op {
            BinaryOp::ShortAnd => {
                let l = self.eval(lhs, src)?;
                if !truth(&l)? {
                    return Ok(Array::bool(false));
                }
                let r = self.eval(rhs, src)?;
                Ok(Array::bool(truth(&r)?))
            }
            BinaryOp::ShortOr => {
                let l = self.eval(lhs, src)?;
                if truth(&l)? {
                    return Ok(Array::bool(true));
                }
                let r = self.eval(rhs, src)?;
                Ok(Array::bool(truth(&r)?))
            }
            _ => {
                let l = self.eval(lhs, src)?;
                let r = self.eval(rhs, src)?;
                ops::binary(op, &l, &r)
            }
        }
    }

    fn eval_range(
        &mut self,
        start: &Expr,
        step: Option<&Expr>,
        stop: &Expr,
        src: &str,
    ) -> Flow<Array> {
        let s = self
            .eval(start, src)?
            .as_f64()
            .ok_or_else(|| Signal::Error(InterpError::msg("range start must be a scalar")))?;
        let e = self
            .eval(stop, src)?
            .as_f64()
            .ok_or_else(|| Signal::Error(InterpError::msg("range stop must be a scalar")))?;
        let st = match step {
            Some(expr) => self
                .eval(expr, src)?
                .as_f64()
                .ok_or_else(|| Signal::Error(InterpError::msg("range step must be a scalar")))?,
            None => 1.0,
        };
        Ok(build_range(s, st, e))
    }

    /// Paren-indexing: variable index (read) or a function call.
    fn eval_index(
        &mut self,
        base: &Expr,
        args: &[Expr],
        nargout: usize,
        src: &str,
        span: Span,
    ) -> Flow<Vec<Array>> {
        // `name(args)` where `name` is not a variable → function call.
        // A builtin constant (e.g. `eps`) is normally not a function, but if a
        // function with that name is registered it takes precedence for the
        // call form `eps('double')`/`eps(x)` (MATLAB's `eps` is both).
        if let ExprKind::Ident(name) = &base.kind
            && !self.context.exists(name)
            && (!is_builtin_constant(name) || self.resolves_function(name))
        {
            let arg_vals = self.eval_args(args, src)?;
            return self.call_function(name, &arg_vals, nargout, src, span);
        }
        // Otherwise index a value.
        let value = self.eval(base, src)?;
        let plan = self.plan_for(&value, args, src)?;
        Ok(vec![index::gather(&value, &plan)?])
    }

    /// Brace-indexing: cell-content extraction (yields possibly several values).
    fn eval_cell_index(&mut self, base: &Expr, args: &[Expr], src: &str) -> Flow<Vec<Array>> {
        let value = self.eval(base, src)?;
        let plan = self.plan_for(&value, args, src)?;
        index::gather_cell_contents(&value, &plan)
    }

    /// Evaluate index arguments against `target`, resolving `:` and `end`.
    fn plan_for(&mut self, target: &Array, args: &[Expr], src: &str) -> Flow<IndexPlan> {
        let dims = target.dims();
        self.plan_for_dims(&dims, args, None, src)
    }

    /// Like [`plan_for`](Self::plan_for) but the target's `dims` are supplied
    /// directly. Used by the in-place indexed-assignment fast path so the index
    /// arguments are evaluated **while the target variable is still bound** (the
    /// index may reference the variable itself, e.g. `v(v > 2) = 0`).
    fn plan_for_dims(
        &mut self,
        dims: &[usize],
        args: &[Expr],
        rhs_dims: Option<&[usize]>,
        src: &str,
    ) -> Flow<IndexPlan> {
        // Stack-allocate the resolved subscripts for the common low-arity case
        // (`A(i)` / `A(i,j)`) — no heap for one or two subscripts.
        let mut resolved: SmallVec<[IndexArg; 2]> = SmallVec::with_capacity(args.len());
        let nargs = args.len();
        for (pos, arg) in args.iter().enumerate() {
            if matches!(arg.kind, ExprKind::Colon) {
                resolved.push(IndexArg::Colon);
            } else {
                let dim_len = end_extent(dims, pos, nargs);
                let v = self.eval_with_end(arg, dim_len, src)?;
                resolved.push(IndexArg::Value(v));
            }
        }
        index::plan_index_rhs(dims, &resolved, rhs_dims)
    }

    /// Evaluate an index argument, substituting `end` with `dim_len`.
    fn eval_with_end(&mut self, e: &Expr, dim_len: usize, src: &str) -> Flow<Array> {
        match &e.kind {
            ExprKind::End => Ok(Array::double(dim_len as f64)),
            ExprKind::Range { start, step, stop } => {
                let s = self.end_scalar(start, dim_len, src)?;
                let e2 = self.end_scalar(stop, dim_len, src)?;
                let st = match step {
                    Some(x) => self.end_scalar(x, dim_len, src)?,
                    None => 1.0,
                };
                Ok(build_range(s, st, e2))
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let l = self.eval_with_end(lhs, dim_len, src)?;
                let r = self.eval_with_end(rhs, dim_len, src)?;
                ops::binary(*op, &l, &r)
            }
            ExprKind::Unary { op, operand } => {
                let v = self.eval_with_end(operand, dim_len, src)?;
                ops::unary(*op, &v)
            }
            _ => self.eval(e, src),
        }
    }

    fn end_scalar(&mut self, e: &Expr, dim_len: usize, src: &str) -> Flow<f64> {
        self.eval_with_end(e, dim_len, src)?
            .as_f64()
            .ok_or_else(|| Signal::Error(InterpError::msg("range bound must be a scalar")))
    }

    fn eval_args(&mut self, args: &[Expr], src: &str) -> Flow<Vec<Array>> {
        let mut out = Vec::with_capacity(args.len());
        for a in args {
            // A `{}`-index argument may expand to multiple values (cs-list);
            // for the minimal Stage 3 set we take the first value.
            if let ExprKind::CellIndex { .. } = &a.kind {
                let vals = self.eval_multi(a, 1, src)?;
                out.extend(vals);
            } else {
                out.push(self.eval(a, src)?);
            }
        }
        Ok(out)
    }

    /// Concatenate already-evaluated `values` along `dim` (1 = vertical,
    /// 2 = horizontal). Exposed for the `cat`/`horzcat`/`vertcat` builtins so
    /// they reuse the same promotion / struct / cell / char concatenation rules
    /// as the `[ ]` matrix-literal path.
    ///
    /// # Errors
    /// Returns a runtime error if the operands' shapes are incompatible.
    pub fn concat_values(&self, dim: usize, values: &[Array]) -> Flow<Array> {
        let refs: Vec<Array> = values.to_vec();
        if dim == 1 { vcat(&refs) } else { hcat(&refs) }
    }

    // ---- Matrix / cell literals ---------------------------------------------

    fn eval_matrix(&mut self, rows: &[Vec<Expr>], src: &str) -> Flow<Array> {
        let mut row_arrays: Vec<Array> = Vec::with_capacity(rows.len());
        for row in rows {
            let mut elems = Vec::with_capacity(row.len());
            for e in row {
                elems.push(self.eval(e, src)?);
            }
            row_arrays.push(hcat(&elems)?);
        }
        vcat(&row_arrays)
    }

    fn eval_cell(&mut self, rows: &[Vec<Expr>], src: &str) -> Flow<Array> {
        if rows.is_empty() || rows.iter().all(Vec::is_empty) {
            return Ok(Array::cell(&[0, 0], Vec::new()));
        }
        let nrows = rows.len();
        let ncols = rows[0].len();
        for r in rows {
            if r.len() != ncols {
                return Err(Signal::Error(InterpError::msg(
                    "cell rows must have equal length",
                )));
            }
        }
        // Column-major flatten.
        let mut data = vec![Array::empty(); nrows * ncols];
        for (ri, row) in rows.iter().enumerate() {
            for (ci, e) in row.iter().enumerate() {
                data[ri + ci * nrows] = self.eval(e, src)?;
            }
        }
        Ok(Array::cell(&[nrows, ncols], data))
    }

    // ---- Function calls -----------------------------------------------------

    /// Whether `name` resolves to a callable function — either a file-local
    /// subfunction of the currently executing file, or a global function.
    fn resolves_function(&self, name: &str) -> bool {
        self.local_funcs_stack
            .last()
            .is_some_and(|l| l.get(name).is_some())
            || self.functions.contains(name)
    }

    /// Call a registered function (builtin or interpreted) by name.
    pub fn call_function(
        &mut self,
        name: &str,
        args: &[Array],
        nargout: usize,
        src: &str,
        span: Span,
    ) -> Flow<Vec<Array>> {
        // Resolve file-local subfunctions first: while a function from a `.m`
        // file is executing, its sibling subfunctions shadow the global table.
        if let Some(locals) = self.local_funcs_stack.last()
            && let Some(def) = locals.get(name).cloned()
        {
            let locals = locals.clone();
            return self.call_interpreted(&def, args, nargout, locals);
        }

        let func = self.functions.get(name).cloned().ok_or_else(|| {
            Signal::Error(
                InterpError::msg(format!("undefined function '{name}'")).located(src, span),
            )
        })?;
        match func {
            Function::Builtin { func, .. } => func(self, args, nargout.max(1)),
            Function::Interpreted { def, locals, .. } => {
                self.call_interpreted(&def, args, nargout, locals)
            }
        }
    }

    /// Run an interpreted `.m` function: bind inputs, execute, collect outputs.
    /// `locals` is the file-local function table that becomes active for the
    /// duration of the call (so sibling subfunctions resolve).
    fn call_interpreted(
        &mut self,
        def: &FunctionDef,
        args: &[Array],
        nargout: usize,
        locals: Arc<crate::function::FileLocals>,
    ) -> Flow<Vec<Array>> {
        if args.len() > def.inputs.len() && !has_varargin(def) {
            return Err(Signal::Error(InterpError::msg(format!(
                "too many input arguments to '{}'",
                def.name
            ))));
        }
        let fsrc = locals.src.clone();
        self.local_funcs_stack.push(locals);
        self.context.push_scope(def.name.clone());
        self.nargin_stack.push(args.len());
        self.nargout_stack.push(nargout.max(1));

        // Bind positional inputs (extra trailing inputs into `varargin`).
        for (i, param) in def.inputs.iter().enumerate() {
            if param.name == "varargin" {
                let rest: Vec<Array> = args.iter().skip(i).cloned().collect();
                let n = rest.len();
                self.context.assign("varargin", Array::cell(&[1, n], rest));
                break;
            }
            if let Some(v) = args.get(i) {
                self.context.assign(&param.name, v.clone());
            }
        }

        let result = self.exec_block(&def.body, &fsrc);

        // On error, unwind the frame before propagating.
        let flow = match result {
            Ok(()) | Err(Signal::Return) => Ok(()),
            Err(Signal::Break | Signal::Continue) => Ok(()),
            Err(e @ Signal::Error(_)) => Err(e),
        };

        let outputs = if flow.is_ok() {
            self.collect_outputs(def, nargout)
        } else {
            Ok(Vec::new())
        };

        self.nargin_stack.pop();
        self.nargout_stack.pop();
        self.context.pop_scope();
        self.local_funcs_stack.pop();

        flow?;
        outputs
    }

    fn collect_outputs(&mut self, def: &FunctionDef, nargout: usize) -> Flow<Vec<Array>> {
        let want = if nargout == 0 { 1 } else { nargout };
        let mut out = Vec::new();
        for (i, oname) in def.outputs.iter().enumerate() {
            if oname == "varargout" {
                if let Some(v) = self.context.lookup("varargout")
                    && let Some(cells) = v.as_cell()
                {
                    out.extend(value::mem_order(cells));
                }
                break;
            }
            if let Some(v) = self.context.lookup(oname) {
                out.push(v.clone());
            } else if i < want {
                return Err(Signal::Error(InterpError::msg(format!(
                    "output argument '{oname}' is not assigned"
                ))));
            } else {
                break;
            }
        }
        Ok(out)
    }

    fn current_nargin(&self) -> usize {
        *self.nargin_stack.last().unwrap_or(&0)
    }

    fn current_nargout(&self) -> usize {
        *self.nargout_stack.last().unwrap_or(&0)
    }

    // ---- Display ------------------------------------------------------------

    fn echo(&mut self, name: &str, value: &Array) {
        let body = value.format(self.format);
        // MATLAB prints `name =\n\n<body>\n` for every value class.
        self.output.push_str(&format!("\n{name} =\n\n{body}\n\n"));
    }
}

// ---- Free helpers -----------------------------------------------------------

/// Unwrap a control-flow signal into an `InterpError` for the public API. A
/// stray break/continue/return at the top is reported as a generic error.
fn unwrap_error(sig: Signal) -> InterpError {
    match sig {
        Signal::Error(e) => e,
        Signal::Break => InterpError::msg("'break' outside of a loop"),
        Signal::Continue => InterpError::msg("'continue' outside of a loop"),
        Signal::Return => InterpError::msg("'return' outside of a function"),
    }
}

/// 1-based line number of byte offset `pos` in `src`.
fn line_of(src: &str, pos: usize) -> usize {
    src[..pos.min(src.len())]
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
        + 1
}

fn is_builtin_constant(name: &str) -> bool {
    matches!(
        name,
        "pi" | "e" | "Inf" | "inf" | "NaN" | "nan" | "eps" | "true" | "false"
    )
}

fn has_varargin(def: &FunctionDef) -> bool {
    def.inputs.iter().any(|p| p.name == "varargin")
}

/// The root variable name of a (possibly nested) l-value chain.
fn root_name(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Ident(n) => Some(n.clone()),
        ExprKind::Index { base, .. }
        | ExprKind::CellIndex { base, .. }
        | ExprKind::Field { base, .. }
        | ExprKind::DynField { base, .. } => root_name(base),
        _ => None,
    }
}

/// The extent to substitute for `end` at index position `pos` of `nargs` total.
fn end_extent(dims: &[usize], pos: usize, nargs: usize) -> usize {
    if nargs == 1 {
        dims.iter().product()
    } else if pos + 1 == nargs {
        // Last subscript collapses remaining dims.
        dims.iter().skip(pos).product::<usize>().max(1)
    } else {
        dims.get(pos).copied().unwrap_or(1)
    }
}

/// Parse a real numeric literal.
fn real_literal(text: &str, single: bool) -> Flow<Array> {
    let v: f64 = text
        .parse()
        .map_err(|_| Signal::Error(InterpError::msg(format!("invalid number '{text}'"))))?;
    Ok(if single {
        Array::single(v as f32)
    } else {
        Array::double(v)
    })
}

/// Parse an imaginary numeric literal (`3i`).
fn imag_literal(text: &str, single: bool) -> Flow<Array> {
    let v: f64 = text
        .parse()
        .map_err(|_| Signal::Error(InterpError::msg(format!("invalid number '{text}'"))))?;
    Ok(if single {
        Array::Scalar(ScalarValue::Complex32(fm_core::C32::new(0.0, v as f32)))
    } else {
        Array::complex64(C64::new(0.0, v))
    })
}

/// Build an inclusive numeric range `start:step:stop` as a row vector.
fn build_range(start: f64, step: f64, stop: f64) -> Array {
    if step == 0.0 || (step > 0.0 && start > stop) || (step < 0.0 && start < stop) {
        return Array::double_matrix(&[1, 0], Vec::new());
    }
    let n = ((stop - start) / step + 1e-10).floor() as i64 + 1;
    let n = n.max(0) as usize;
    let data: Vec<f64> = (0..n).map(|i| start + step * i as f64).collect();
    if data.len() == 1 {
        Array::double(data[0])
    } else {
        Array::double_matrix(&[1, data.len()], data)
    }
}

/// The columns of a value, each as an [`Array`] (for `for` iteration).
fn columns_of(v: &Array) -> Vec<Array> {
    let dims = v.shape();
    if dims.len() < 2 {
        return vec![v.clone()];
    }
    let rows = dims[0];
    let cols: usize = dims.iter().skip(1).product();
    if v.numel() == 0 {
        return Vec::new();
    }
    let needed_dims = Dims::from_slice(dims);
    let mut out = Vec::with_capacity(cols);
    for c in 0..cols {
        let positions: SmallVec<[usize; 4]> = (0..rows).map(|r| r + c * rows).collect();
        let plan = IndexPlan {
            result_dims: SmallVec::from_slice(&[rows, 1]),
            needed_dims: needed_dims.clone(),
            linear: positions,
            deleted_axis: None,
        };
        out.push(index::gather(v, &plan).unwrap_or_else(|_| v.clone()));
    }
    out
}

/// Whether `value` matches a `switch` case `label` (string or numeric, or a
/// cell of alternatives).
fn switch_matches(value: &Array, label: &Array) -> bool {
    if let Some(cells) = label.as_cell() {
        return cells.iter().any(|c| switch_matches(value, c));
    }
    if value.is_char() || label.is_char() {
        return value.as_string() == label.as_string();
    }
    // Numeric (possibly complex) scalar comparison: compare as complex doubles
    // so `1.0f + i` (complex single) matches `1.0 + i` (complex double) — the
    // `as_f64` path would discard the imaginary component.
    if value.is_complex() || label.is_complex() {
        let v = value_c64(value);
        let l = value_c64(label);
        return matches!((v, l), (Some(a), Some(b)) if a == b);
    }
    match (value.as_f64(), label.as_f64()) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// The single complex value of a scalar array (`None` if not numel-1).
fn value_c64(a: &Array) -> Option<C64> {
    if a.numel() != 1 {
        return None;
    }
    value::to_c64_vec(a).first().copied()
}

/// Element-wise transpose (2-D only); `conjugate` negates imaginary parts.
fn transpose(v: &Array, conjugate: bool) -> Flow<Array> {
    // Sparse transpose stays sparse (CSC transpose). Conjugate transpose of a
    // complex sparse would negate the imaginary part; the corpus only transposes
    // real sparse, so densify the rare conjugate-complex case for fidelity.
    if let Some(s) = v.as_sparse()
        && !(conjugate && s.is_complex())
    {
        return Ok(Array::sparse(s.transpose()));
    }
    let dims = v.dims();
    if dims.len() != 2 {
        return Err(Signal::Error(InterpError::msg(
            "transpose requires a 2-D array",
        )));
    }
    let (r, c) = (dims[0], dims[1]);
    // Map the [c, r] transpose result into column-major source positions:
    // result[new_i, new_j] = source[new_j, new_i].
    let mut linear: SmallVec<[usize; 4]> = SmallVec::from_elem(0usize, r * c);
    for new_j in 0..r {
        for new_i in 0..c {
            linear[new_i + new_j * c] = new_j + new_i * r;
        }
    }
    let plan = IndexPlan {
        result_dims: SmallVec::from_slice(&[c, r]),
        needed_dims: SmallVec::from_slice(&[c, r]),
        linear,
        deleted_axis: None,
    };
    let mut out = index::gather(v, &plan)?;
    if conjugate && out.is_complex() {
        let d = out.dims();
        let data = value::to_c64_vec(&out)
            .into_iter()
            .map(|z| z.conj())
            .collect();
        out = value::build_complex(&d, data);
    }
    Ok(out)
}

/// Horizontal concatenation of a row of values (`[a b c]`).
fn hcat(elems: &[Array]) -> Flow<Array> {
    let non_empty: Vec<&Array> = elems.iter().filter(|e| e.numel() > 0).collect();
    if non_empty.is_empty() {
        return Ok(Array::empty());
    }
    if non_empty.len() == 1 {
        return Ok(non_empty[0].clone());
    }
    let rows = non_empty[0].dims()[0];
    let any_char = non_empty.iter().any(|e| e.is_char());
    for e in &non_empty {
        if e.dims()[0] != rows {
            return Err(Signal::Error(InterpError::msg(
                "horizontal concatenation: row counts must match",
            )));
        }
    }
    if any_char && non_empty.iter().all(|e| e.is_char()) {
        // Concatenate char row-strings.
        let mut s = String::new();
        for e in &non_empty {
            s.push_str(&e.as_string().unwrap_or_default());
        }
        return Ok(Array::char_string(&s));
    }
    let any_cell = non_empty.iter().any(|e| matches!(e, Array::Cell(_)));
    if any_cell {
        return concat_cells(&non_empty, true);
    }
    if non_empty.iter().any(|e| matches!(e, Array::Struct(_))) {
        return concat_structs(&non_empty, true);
    }
    let complex = non_empty.iter().any(|e| e.is_complex());
    let total_cols: usize = non_empty.iter().map(|e| e.dims()[1]).sum();
    let class = result_class(&non_empty);
    if complex {
        let mut data = Vec::new();
        for e in &non_empty {
            data.extend(value::to_c64_vec(e));
        }
        return Ok(value::build_complex_class(class, &[rows, total_cols], data));
    }
    let mut data = Vec::new();
    for e in &non_empty {
        data.extend(value::to_f64_vec(e));
    }
    Ok(value::build_real(class, &[rows, total_cols], data))
}

/// Vertical concatenation of rows (`[r1; r2; r3]`).
fn vcat(rows: &[Array]) -> Flow<Array> {
    let non_empty: Vec<&Array> = rows.iter().filter(|e| e.numel() > 0).collect();
    if non_empty.is_empty() {
        return Ok(Array::empty());
    }
    if non_empty.len() == 1 {
        return Ok(non_empty[0].clone());
    }
    let cols = non_empty[0].dims()[1];
    for e in &non_empty {
        if e.dims()[1] != cols {
            return Err(Signal::Error(InterpError::msg(
                "vertical concatenation: column counts must match",
            )));
        }
    }
    if non_empty.iter().all(|e| e.is_char()) && non_empty.iter().any(|e| e.is_char()) {
        // Build a 2-D char array (rows of equal length).
        let total_rows: usize = non_empty.iter().map(|e| e.dims()[0]).sum();
        let mut chars = vec!['\u{0}'; total_rows * cols];
        let mut row_off = 0;
        for e in &non_empty {
            let er = e.dims()[0];
            let flat: Vec<char> = match e {
                Array::Char(d) => value::mem_order(d),
                Array::Scalar(ScalarValue::Char(c)) => vec![*c],
                _ => Vec::new(),
            };
            for j in 0..cols {
                for i in 0..er {
                    chars[(row_off + i) + j * total_rows] = flat[i + j * er];
                }
            }
            row_off += er;
        }
        return Ok(value::char_matrix(&[total_rows, cols], chars));
    }
    if non_empty.iter().any(|e| matches!(e, Array::Cell(_))) {
        return concat_cells(&non_empty, false);
    }
    if non_empty.iter().any(|e| matches!(e, Array::Struct(_))) {
        return concat_structs(&non_empty, false);
    }
    let total_rows: usize = non_empty.iter().map(|e| e.dims()[0]).sum();
    let complex = non_empty.iter().any(|e| e.is_complex());
    let class = result_class(&non_empty);
    if complex {
        let mut data = vec![C64::new(0.0, 0.0); total_rows * cols];
        let mut row_off = 0;
        for e in &non_empty {
            let er = e.dims()[0];
            let flat = value::to_c64_vec(e);
            for j in 0..cols {
                for i in 0..er {
                    data[(row_off + i) + j * total_rows] = flat[i + j * er];
                }
            }
            row_off += er;
        }
        return Ok(value::build_complex_class(class, &[total_rows, cols], data));
    }
    let mut data = vec![0.0f64; total_rows * cols];
    let mut row_off = 0;
    for e in &non_empty {
        let er = e.dims()[0];
        let flat = value::to_f64_vec(e);
        for j in 0..cols {
            for i in 0..er {
                data[(row_off + i) + j * total_rows] = flat[i + j * er];
            }
        }
        row_off += er;
    }
    Ok(value::build_real(class, &[total_rows, cols], data))
}

/// Concatenate cell arrays (horizontally or vertically).
fn concat_cells(elems: &[&Array], horizontal: bool) -> Flow<Array> {
    // Wrap any non-cell into a 1x1 cell, then concat element vectors.
    let mut blocks: Vec<(Vec<usize>, Vec<Array>)> = Vec::new();
    for e in elems {
        match e.as_cell() {
            Some(c) => blocks.push((e.dims(), value::mem_order(c))),
            None => blocks.push((vec![1, 1], vec![(*e).clone()])),
        }
    }
    if horizontal {
        let rows = blocks[0].0[0];
        let total_cols: usize = blocks.iter().map(|(d, _)| d[1]).sum();
        let mut data = Vec::with_capacity(rows * total_cols);
        for (_, v) in &blocks {
            data.extend(v.iter().cloned());
        }
        Ok(Array::cell(&[rows, total_cols], data))
    } else {
        let cols = blocks[0].0[1];
        let total_rows: usize = blocks.iter().map(|(d, _)| d[0]).sum();
        let mut data = vec![Array::empty(); total_rows * cols];
        let mut row_off = 0;
        for (d, v) in &blocks {
            let er = d[0];
            for j in 0..cols {
                for i in 0..er {
                    data[(row_off + i) + j * total_rows] = v[i + j * er].clone();
                }
            }
            row_off += er;
        }
        Ok(Array::cell(&[total_rows, cols], data))
    }
}

/// Concatenate struct arrays (`[a, b]` / `[a; b]`). Every element must be a
/// struct with the **same set** of field names (order may differ — MATLAB
/// reorders the second operand to match the first). Fields are taken from the
/// first block's order.
fn concat_structs(elems: &[&Array], horizontal: bool) -> Flow<Array> {
    let mut structs = Vec::with_capacity(elems.len());
    for e in elems {
        let s = e.as_struct().ok_or_else(|| {
            Signal::Error(InterpError::msg(
                "cannot concatenate a struct with a non-struct value",
            ))
        })?;
        structs.push(s);
    }
    // Field-name union: first block's order, then any new names from the rest.
    // FreeMat permits concatenating structs with differing fields, filling the
    // missing ones with empty (`[]`).
    let mut names: Vec<String> = structs[0].field_name_strings();
    for s in &structs {
        for n in s.field_name_strings() {
            if !names.contains(&n) {
                names.push(n);
            }
        }
    }

    let blocks: Vec<(Vec<usize>, &&fm_core::StructArray)> =
        structs.iter().map(|s| (s.dims().to_vec(), s)).collect();

    let (out_dims, order): (Vec<usize>, Vec<(usize, usize)>) = if horizontal {
        let rows = blocks[0].0[0];
        let total_cols: usize = blocks.iter().map(|(d, _)| d[1]).sum();
        // Column-major element order over the concatenated [rows, total_cols].
        let mut order = Vec::with_capacity(rows * total_cols);
        for (bi, (d, _)) in blocks.iter().enumerate() {
            let bc = d[1];
            for j in 0..bc {
                for i in 0..rows {
                    order.push((bi, i + j * rows));
                }
            }
        }
        (vec![rows, total_cols], order)
    } else {
        let cols = blocks[0].0[1];
        let total_rows: usize = blocks.iter().map(|(d, _)| d[0]).sum();
        let mut order = vec![(0usize, 0usize); total_rows * cols];
        let mut row_off = 0;
        for (bi, (d, _)) in blocks.iter().enumerate() {
            let br = d[0];
            for j in 0..cols {
                for i in 0..br {
                    order[(row_off + i) + j * total_rows] = (bi, i + j * br);
                }
            }
            row_off += br;
        }
        (vec![total_rows, cols], order)
    };

    let fields: Vec<(String, Vec<Array>)> = names
        .iter()
        .map(|name| {
            let col: Vec<Array> = order
                .iter()
                .map(|&(bi, ei)| {
                    structs[bi]
                        .field(name)
                        .and_then(|v| v.get(ei))
                        .cloned()
                        .unwrap_or_else(Array::empty)
                })
                .collect();
            (name.clone(), col)
        })
        .collect();

    Ok(Array::struct_array(fm_core::StructArray::from_fields(
        out_dims, fields,
    )))
}

/// The common result class for a concatenation, mirroring FreeMat's
/// `ComputeCatType`: the **first** integer class found wins; otherwise any
/// `single` makes the result `single`; otherwise any `double` makes it
/// `double`; otherwise `bool`. (Char/cell/struct cases are handled by the
/// caller before this is reached.)
fn result_class(elems: &[&Array]) -> DataClass {
    // First integer class found dominates.
    for e in elems {
        let c = e.class();
        if c.is_integer() {
            return c;
        }
    }
    if elems.iter().any(|e| e.class() == DataClass::Float) {
        DataClass::Float
    } else if elems.iter().any(|e| e.class() == DataClass::Double) {
        DataClass::Double
    } else {
        DataClass::Bool
    }
}
