//! Interpreter-held graphics state: the retained [`Scene`], the current figure
//! id, a **handle registry** (the Stage 7.5 set/get property system), and an
//! optional [`GraphicsSink`] the webserver installs to broadcast scene updates.
//!
//! The sink is **optional** — when none is installed (library tests, the
//! conformance harness), graphics builtins still mutate the retained scene; they
//! just don't publish. So nothing here pulls in `axum`/`tokio`.
//!
//! # Handle model
//! Graphics objects are addressed by MATLAB-style numeric handles (stored as
//! `f64` in user space). Figures use their figure *number* (small integers).
//! Child objects (axes, lines, …) get larger handle ids allocated from
//! [`GraphicsState::next_handle`]. A handle resolves to a location in the scene
//! via [`GraphicsState::resolve`]; unknown (free-form) properties set with
//! `set` are echoed back by `get` from a per-handle [`ObjectRecord::extra`] bag.

use std::collections::BTreeMap;
use std::collections::HashMap;

use fm_core::Array;
use fm_graphics::{Axes, Figure, GraphicsSink, Scene, Series};

/// The kind of a registered graphics object (drives `get(h,'type')` and
/// `ishandle(h,'axes')`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjKind {
    /// The root object (handle `0`); parent of all figures.
    Root,
    /// A figure window.
    Figure,
    /// A coordinate system (an axes).
    Axes,
    /// A line / lineseries child of an axes.
    Line,
    /// A surface child of an axes.
    Surface,
    /// An image child of an axes.
    Image,
    /// A contour child of an axes.
    Contour,
    /// A patch / filled-polygon child of an axes (`patch`).
    Patch,
    /// A text object (title / label) — minimal support.
    Text,
}

impl ObjKind {
    /// The MATLAB `type` string for this kind.
    #[must_use]
    pub fn type_name(self) -> &'static str {
        match self {
            ObjKind::Root => "root",
            ObjKind::Figure => "figure",
            ObjKind::Axes => "axes",
            ObjKind::Line => "line",
            ObjKind::Surface => "surface",
            ObjKind::Image => "image",
            ObjKind::Contour => "contour",
            ObjKind::Patch => "patch",
            ObjKind::Text => "text",
        }
    }
}

/// Where a handle's object lives in the retained scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjLocation {
    /// A figure (by id).
    Figure { fig: u64 },
    /// An axes (figure id + index within `Figure::axes`).
    Axes { fig: u64, axes: usize },
    /// A series child (figure id + axes index + series index).
    Series {
        /// Owning figure id.
        fig: u64,
        /// Index within the figure's axes vector.
        axes: usize,
        /// Index within the axes' series vector.
        series: usize,
    },
    /// A text annotation child (figure id + axes index + index within the
    /// axes' `texts` vector).
    Text {
        /// Owning figure id.
        fig: u64,
        /// Index within the figure's axes vector.
        axes: usize,
        /// Index within the axes' `texts` vector.
        idx: usize,
    },
}

/// A registry entry: kind, location, and a free-form property bag for unknown
/// (non-modeled) properties so `set`/`get` round-trip arbitrary names.
#[derive(Debug, Clone)]
pub struct ObjectRecord {
    /// What kind of object this is.
    pub kind: ObjKind,
    /// Where it lives in the scene.
    pub location: ObjLocation,
    /// Free-form properties stored verbatim (echoed by `get`).
    pub extra: BTreeMap<String, Array>,
}

/// All graphics state owned by the interpreter.
#[derive(Default)]
pub struct GraphicsState {
    /// The retained scene (every open figure).
    pub scene: Scene,
    /// The current figure id (`gcf`); `0` = no figure yet.
    pub current_figure: u64,
    /// True once a plotting command has mutated the scene since the last flush
    /// (so an implicit draw after a non-suppressed command knows to publish).
    pub dirty: bool,
    /// The optional broadcast sink (installed by `fm-cli`'s webserver).
    pub sink: Option<Box<dyn GraphicsSink>>,
    /// Handle registry: handle id → object record.
    objects: HashMap<u64, ObjectRecord>,
    /// Next child-object handle to allocate (figures use their figure number).
    next_handle: u64,
    /// The current object handle (`gco`); `0` = none.
    pub current_object: u64,
}

impl GraphicsState {
    /// Ensure there is a current figure, creating figure 1 if none exists, and
    /// return its id.
    pub fn ensure_figure(&mut self) -> u64 {
        if self.current_figure == 0 {
            self.select_figure(1);
        } else if self.scene.figure(self.current_figure).is_none() {
            self.select_figure(self.current_figure);
        }
        self.current_figure
    }

    /// Select (or create) a figure by id and make it current. A freshly created
    /// figure starts with one registered default axes.
    pub fn select_figure(&mut self, id: u64) {
        self.current_figure = id;
        let existed = self.scene.figure(id).is_some();
        self.scene.figure_mut_or_insert(id);
        self.objects.entry(id).or_insert(ObjectRecord {
            kind: ObjKind::Figure,
            location: ObjLocation::Figure { fig: id },
            extra: BTreeMap::new(),
        });
        if !existed {
            // Register the default axes the figure was created with.
            self.register_existing_axes(id);
        }
    }

    /// The next unused figure id (max existing + 1, or 1).
    #[must_use]
    pub fn next_figure_id(&self) -> u64 {
        self.scene.figures.iter().map(|f| f.id).max().unwrap_or(0) + 1
    }

    /// Mutable access to the current figure (creating one if needed).
    pub fn current_figure_mut(&mut self) -> &mut Figure {
        let id = self.ensure_figure();
        self.scene.figure_mut_or_insert(id)
    }

    /// Allocate a fresh child-object handle (>= 1000 to avoid colliding with
    /// figure numbers).
    fn alloc_handle(&mut self) -> u64 {
        if self.next_handle < 1000 {
            self.next_handle = 1000;
        }
        let h = self.next_handle;
        self.next_handle += 1;
        h
    }

    /// Register any axes in the figure that lack a handle, assigning fresh ids.
    fn register_existing_axes(&mut self, fig: u64) {
        let count = self
            .scene
            .figure(fig)
            .map(|f| f.axes.len())
            .unwrap_or_default();
        for idx in 0..count {
            let needs = self
                .scene
                .figure(fig)
                .and_then(|f| f.axes.get(idx))
                .map(|a| a.handle == 0)
                .unwrap_or(false);
            if needs {
                let h = self.alloc_handle();
                if let Some(ax) = self.scene.figure_mut_or_insert(fig).axes.get_mut(idx)
                    && ax.handle == 0
                {
                    ax.handle = h;
                }
                self.objects.insert(
                    h,
                    ObjectRecord {
                        kind: ObjKind::Axes,
                        location: ObjLocation::Axes { fig, axes: idx },
                        extra: BTreeMap::new(),
                    },
                );
            }
        }
    }

    /// The handle of the current axes of the current figure (registering it if
    /// necessary). This is `gca`.
    pub fn current_axes_handle(&mut self) -> u64 {
        let fig = self.ensure_figure();
        self.register_existing_axes(fig);
        let idx = self.scene.figure(fig).map(|f| f.current_axes).unwrap_or(0);
        self.scene
            .figure(fig)
            .and_then(|f| f.axes.get(idx))
            .map(|a| a.handle)
            .unwrap_or(0)
    }

    /// The handle of the current figure (registering it). This is `gcf`.
    pub fn current_figure_handle(&mut self) -> u64 {
        self.ensure_figure()
    }

    /// Look up an object record by handle.
    #[must_use]
    pub fn object(&self, handle: u64) -> Option<&ObjectRecord> {
        self.objects.get(&handle)
    }

    /// True if `handle` names a live graphics object. Handle `0` is the root
    /// object, which always exists.
    #[must_use]
    pub fn is_handle(&self, handle: u64) -> bool {
        handle == 0 || self.objects.contains_key(&handle)
    }

    /// The kind of the object named by `handle`, if any. Handle `0` is the root.
    #[must_use]
    pub fn kind_of(&self, handle: u64) -> Option<ObjKind> {
        if handle == 0 {
            return Some(ObjKind::Root);
        }
        self.objects.get(&handle).map(|r| r.kind)
    }

    /// The parent handle of `handle` in the object hierarchy:
    /// figure → root (`0`); axes → its figure; series/text → its axes; the root
    /// has no parent (`None`).
    #[must_use]
    pub fn parent(&self, handle: u64) -> Option<u64> {
        if handle == 0 {
            return None; // the root has no parent.
        }
        match self.resolve(handle)? {
            ObjLocation::Figure { .. } => Some(0),
            ObjLocation::Axes { fig, .. } => Some(fig),
            ObjLocation::Series { fig, axes, .. } | ObjLocation::Text { fig, axes, .. } => self
                .scene
                .figure(fig)
                .and_then(|f| f.axes.get(axes))
                .map(|a| a.handle),
        }
    }

    /// The child handles of `handle`, in a deterministic (sorted-ascending)
    /// order: root → all figure handles; figure → its axes handles; axes → its
    /// series/text children; a leaf object → empty.
    #[must_use]
    pub fn children(&self, handle: u64) -> Vec<u64> {
        let mut out: Vec<u64> = if handle == 0 {
            // The root: every figure.
            self.objects
                .iter()
                .filter(|(_, r)| matches!(r.location, ObjLocation::Figure { .. }))
                .map(|(&h, _)| h)
                .collect()
        } else {
            match self.resolve(handle) {
                Some(ObjLocation::Figure { fig }) => self
                    .objects
                    .iter()
                    .filter(|(_, r)| matches!(r.location, ObjLocation::Axes { fig: f, .. } if f == fig))
                    .map(|(&h, _)| h)
                    .collect(),
                Some(ObjLocation::Axes { fig, axes }) => self
                    .objects
                    .iter()
                    .filter(|(_, r)| {
                        matches!(r.location, ObjLocation::Series { fig: f, axes: a, .. } if f == fig && a == axes)
                            || matches!(r.location, ObjLocation::Text { fig: f, axes: a, .. } if f == fig && a == axes)
                    })
                    .map(|(&h, _)| h)
                    .collect(),
                _ => Vec::new(),
            }
        };
        out.sort_unstable();
        out
    }

    /// Resolve a handle to its current location (the location may have shifted
    /// if series were added/removed; locations stay stable for axes/figures).
    #[must_use]
    pub fn resolve(&self, handle: u64) -> Option<ObjLocation> {
        self.objects.get(&handle).map(|r| r.location)
    }

    /// Create a new axes occupying `position` in the current figure, register
    /// it, make it current, and return its handle.
    pub fn create_axes(&mut self, position: [f64; 4]) -> u64 {
        let fig = self.ensure_figure();
        let h = self.alloc_handle();
        let mut ax = Axes::with_position(position);
        ax.handle = h;
        let figure = self.scene.figure_mut_or_insert(fig);
        figure.axes.push(ax);
        let idx = figure.axes.len() - 1;
        figure.current_axes = idx;
        self.objects.insert(
            h,
            ObjectRecord {
                kind: ObjKind::Axes,
                location: ObjLocation::Axes { fig, axes: idx },
                extra: BTreeMap::new(),
            },
        );
        self.current_object = h;
        self.dirty = true;
        h
    }

    /// Make the axes named by `handle` the current axes of its figure.
    /// Returns `true` if the handle named an axes.
    pub fn select_axes(&mut self, handle: u64) -> bool {
        if let Some(ObjLocation::Axes { fig, axes }) = self.resolve(handle) {
            self.current_figure = fig;
            if let Some(figure) = self.scene.figures.iter_mut().find(|f| f.id == fig) {
                figure.current_axes = axes;
            }
            self.current_object = handle;
            true
        } else {
            false
        }
    }

    /// Register a newly appended series in `(fig, axes)` as a child handle of
    /// the given kind, and return its handle.
    pub fn register_series(&mut self, fig: u64, axes: usize, kind: ObjKind) -> u64 {
        let series = self
            .scene
            .figure(fig)
            .and_then(|f| f.axes.get(axes))
            .map(|a| a.series.len().saturating_sub(1))
            .unwrap_or(0);
        let h = self.alloc_handle();
        self.objects.insert(
            h,
            ObjectRecord {
                kind,
                location: ObjLocation::Series { fig, axes, series },
                extra: BTreeMap::new(),
            },
        );
        self.current_object = h;
        h
    }

    /// Register a newly appended text annotation in `(fig, axes)` (the last entry
    /// of the axes' `texts` vector) as a [`ObjKind::Text`] child handle, and
    /// return its handle.
    pub fn register_text(&mut self, fig: u64, axes: usize) -> u64 {
        let idx = self
            .scene
            .figure(fig)
            .and_then(|f| f.axes.get(axes))
            .map(|a| a.texts.len().saturating_sub(1))
            .unwrap_or(0);
        let h = self.alloc_handle();
        self.objects.insert(
            h,
            ObjectRecord {
                kind: ObjKind::Text,
                location: ObjLocation::Text { fig, axes, idx },
                extra: BTreeMap::new(),
            },
        );
        self.current_object = h;
        h
    }

    /// Delete an object by handle: a figure removes the whole figure window; an
    /// axes removes the axes; a series removes the series. Child handles are
    /// dropped from the registry. Returns `true` if anything was deleted.
    pub fn delete(&mut self, handle: u64) -> bool {
        let Some(loc) = self.resolve(handle) else {
            return false;
        };
        match loc {
            ObjLocation::Figure { fig } => {
                self.scene.figures.retain(|f| f.id != fig);
                self.objects
                    .retain(|_, r| !location_in_figure(r.location, fig));
                if self.current_figure == fig {
                    self.current_figure = self.scene.figures.last().map(|f| f.id).unwrap_or(0);
                }
            }
            ObjLocation::Axes { fig, axes } => {
                if let Some(figure) = self.scene.figures.iter_mut().find(|f| f.id == fig)
                    && axes < figure.axes.len()
                {
                    figure.axes.remove(axes);
                    if figure.current_axes >= figure.axes.len() {
                        figure.current_axes = figure.axes.len().saturating_sub(1);
                    }
                }
                // Drop this axes and its series children, then reindex.
                self.objects
                    .retain(|_, r| !location_is_axes_or_child(r.location, fig, axes));
                self.reindex_after_axes_removal(fig, axes);
            }
            ObjLocation::Series { fig, axes, series } => {
                if let Some(s) = self
                    .scene
                    .figures
                    .iter_mut()
                    .find(|f| f.id == fig)
                    .and_then(|f| f.axes.get_mut(axes))
                    && series < s.series.len()
                {
                    s.series.remove(series);
                }
                self.objects.remove(&handle);
                self.reindex_after_series_removal(fig, axes, series);
            }
            ObjLocation::Text { fig, axes, idx } => {
                if let Some(a) = self
                    .scene
                    .figures
                    .iter_mut()
                    .find(|f| f.id == fig)
                    .and_then(|f| f.axes.get_mut(axes))
                    && idx < a.texts.len()
                {
                    a.texts.remove(idx);
                }
                self.objects.remove(&handle);
                self.reindex_after_text_removal(fig, axes, idx);
            }
        }
        self.dirty = true;
        true
    }

    /// After removing axes `removed` from `fig`, shift the stored indices of the
    /// later axes (and their series children) down by one.
    fn reindex_after_axes_removal(&mut self, fig: u64, removed: usize) {
        for rec in self.objects.values_mut() {
            match &mut rec.location {
                ObjLocation::Axes { fig: f, axes } if *f == fig && *axes > removed => *axes -= 1,
                ObjLocation::Series { fig: f, axes, .. }
                | ObjLocation::Text { fig: f, axes, .. }
                    if *f == fig && *axes > removed =>
                {
                    *axes -= 1;
                }
                _ => {}
            }
        }
    }

    /// After removing series `removed` from `(fig, axes)`, shift later series
    /// indices down by one.
    fn reindex_after_series_removal(&mut self, fig: u64, axes: usize, removed: usize) {
        for rec in self.objects.values_mut() {
            if let ObjLocation::Series {
                fig: f,
                axes: a,
                series,
            } = &mut rec.location
                && *f == fig
                && *a == axes
                && *series > removed
            {
                *series -= 1;
            }
        }
    }

    /// After removing text `removed` from `(fig, axes)`, shift later text
    /// indices down by one.
    fn reindex_after_text_removal(&mut self, fig: u64, axes: usize, removed: usize) {
        for rec in self.objects.values_mut() {
            if let ObjLocation::Text {
                fig: f,
                axes: a,
                idx,
            } = &mut rec.location
                && *f == fig
                && *a == axes
                && *idx > removed
            {
                *idx -= 1;
            }
        }
    }

    /// Reset a figure to a single fresh default axes (`clf`), dropping all of
    /// its axes/series children from the registry.
    pub fn delete_children_of_figure(&mut self, fig: u64) {
        self.objects
            .retain(|_, r| !matches!(r.location, ObjLocation::Axes { fig: f, .. } | ObjLocation::Series { fig: f, .. } | ObjLocation::Text { fig: f, .. } if f == fig));
        if let Some(figure) = self.scene.figures.iter_mut().find(|f| f.id == fig) {
            figure.axes = vec![Axes::new()];
            figure.current_axes = 0;
        }
        self.register_existing_axes(fig);
        self.current_object = 0;
    }

    /// Clear all figures (`close all`) and the registry.
    pub fn close_all(&mut self) {
        self.scene.figures.clear();
        self.objects.clear();
        self.current_figure = 0;
        self.current_object = 0;
        self.dirty = true;
    }

    /// Mutable access to a series by location, if it still exists.
    pub fn series_at_mut(&mut self, loc: ObjLocation) -> Option<&mut Series> {
        if let ObjLocation::Series { fig, axes, series } = loc {
            self.scene
                .figures
                .iter_mut()
                .find(|f| f.id == fig)?
                .axes
                .get_mut(axes)?
                .series
                .get_mut(series)
        } else {
            None
        }
    }

    /// Mutable access to an axes by location, if it still exists.
    pub fn axes_at_mut(&mut self, loc: ObjLocation) -> Option<&mut Axes> {
        match loc {
            ObjLocation::Axes { fig, axes }
            | ObjLocation::Series { fig, axes, .. }
            | ObjLocation::Text { fig, axes, .. } => self
                .scene
                .figures
                .iter_mut()
                .find(|f| f.id == fig)?
                .axes
                .get_mut(axes),
            ObjLocation::Figure { .. } => None,
        }
    }

    /// Mutable access to a text annotation by location, if it still exists.
    pub fn text_at_mut(&mut self, loc: ObjLocation) -> Option<&mut fm_graphics::TextLabel> {
        if let ObjLocation::Text { fig, axes, idx } = loc {
            self.scene
                .figures
                .iter_mut()
                .find(|f| f.id == fig)?
                .axes
                .get_mut(axes)?
                .texts
                .get_mut(idx)
        } else {
            None
        }
    }

    /// Store a free-form (non-modeled) property on a handle's bag.
    pub fn set_extra(&mut self, handle: u64, name: &str, value: Array) {
        if let Some(rec) = self.objects.get_mut(&handle) {
            rec.extra.insert(name.to_ascii_lowercase(), value);
        }
    }

    /// Read a free-form property previously stored on a handle's bag.
    #[must_use]
    pub fn get_extra(&self, handle: u64, name: &str) -> Option<Array> {
        self.objects
            .get(&handle)
            .and_then(|r| r.extra.get(&name.to_ascii_lowercase()))
            .cloned()
    }

    /// Every live handle (for `findobj` with no filter).
    #[must_use]
    pub fn all_handles(&self) -> Vec<u64> {
        let mut v: Vec<u64> = self.objects.keys().copied().collect();
        v.sort_unstable();
        v
    }

    /// The free-form property bag of a handle (for `copyobj` extra-copy).
    #[must_use]
    pub fn extra_of(&self, handle: u64) -> Option<&BTreeMap<String, Array>> {
        self.objects.get(&handle).map(|r| &r.extra)
    }

    /// Raise figure `fig` in the headless z-order: move it to the end of
    /// `scene.figures` (the "front") and make it the current figure. Returns
    /// `true` if the figure exists.
    pub fn raise_figure(&mut self, fig: u64) -> bool {
        if let Some(pos) = self.scene.figures.iter().position(|f| f.id == fig) {
            let f = self.scene.figures.remove(pos);
            self.scene.figures.push(f);
            self.current_figure = fig;
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Lower figure `fig` in the headless z-order: move it to the front of the
    /// `scene.figures` vector (the "back"). Returns `true` if it exists.
    pub fn lower_figure(&mut self, fig: u64) -> bool {
        if let Some(pos) = self.scene.figures.iter().position(|f| f.id == fig) {
            let f = self.scene.figures.remove(pos);
            self.scene.figures.insert(0, f);
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Deep-copy the series/text named by `src` into the axes `(fig, axes)`,
    /// returning the new child handle (carrying the same kind and `extra` bag).
    /// Returns `None` if `src` is not a series/text or the target axes is gone.
    pub fn copy_child_into_axes(&mut self, src: u64, fig: u64, axes: usize) -> Option<u64> {
        let kind = self.kind_of(src)?;
        let loc = self.resolve(src)?;
        let extra = self.objects.get(&src).map(|r| r.extra.clone())?;
        match loc {
            ObjLocation::Series { .. } => {
                let cloned = self.series_at(loc)?.clone();
                let target = self.scene.figures.iter_mut().find(|f| f.id == fig)?;
                let ax = target.axes.get_mut(axes)?;
                ax.series.push(cloned);
                let h = self.register_series(fig, axes, kind);
                if let Some(rec) = self.objects.get_mut(&h) {
                    rec.extra = extra;
                }
                Some(h)
            }
            ObjLocation::Text { .. } => {
                let cloned = self.text_at(loc)?.clone();
                let target = self.scene.figures.iter_mut().find(|f| f.id == fig)?;
                let ax = target.axes.get_mut(axes)?;
                ax.texts.push(cloned);
                let h = self.register_text(fig, axes);
                if let Some(rec) = self.objects.get_mut(&h) {
                    rec.extra = extra;
                }
                Some(h)
            }
            _ => None,
        }
    }

    /// Deep-copy the axes named by `src` into figure `fig`, recursively copying
    /// its series/text children. Returns the new axes handle, or `None`.
    pub fn copy_axes_into_figure(&mut self, src: u64, fig: u64) -> Option<u64> {
        let ObjLocation::Axes {
            fig: sfig,
            axes: saxes,
        } = self.resolve(src)?
        else {
            return None;
        };
        let extra = self.objects.get(&src).map(|r| r.extra.clone())?;
        let mut cloned = self
            .scene
            .figure(sfig)
            .and_then(|f| f.axes.get(saxes))
            .cloned()?;
        // The clone gets a fresh handle (assigned below) and its child handles
        // are re-registered, so clear any stale references first.
        cloned.handle = 0;
        let h = self.alloc_handle();
        cloned.handle = h;
        let target = self.scene.figure_mut_or_insert(fig);
        target.axes.push(cloned);
        let new_idx = target.axes.len() - 1;
        target.current_axes = new_idx;
        self.objects.insert(
            h,
            ObjectRecord {
                kind: ObjKind::Axes,
                location: ObjLocation::Axes { fig, axes: new_idx },
                extra,
            },
        );
        // Register handles for the copied series/text children (they came along
        // inside the cloned Axes; their old handles do not transfer).
        let (n_series, n_text) = self
            .scene
            .figure(fig)
            .and_then(|f| f.axes.get(new_idx))
            .map(|a| (a.series.len(), a.texts.len()))
            .unwrap_or((0, 0));
        // Map each copied child to the kind of the corresponding source child.
        let src_series_kinds: Vec<ObjKind> = (0..n_series)
            .map(|s| self.kind_of_series_at(sfig, saxes, s))
            .collect();
        for s in 0..n_series {
            let kind = src_series_kinds.get(s).copied().unwrap_or(ObjKind::Line);
            let ch = self.alloc_handle();
            self.objects.insert(
                ch,
                ObjectRecord {
                    kind,
                    location: ObjLocation::Series {
                        fig,
                        axes: new_idx,
                        series: s,
                    },
                    extra: BTreeMap::new(),
                },
            );
        }
        for t in 0..n_text {
            let ch = self.alloc_handle();
            self.objects.insert(
                ch,
                ObjectRecord {
                    kind: ObjKind::Text,
                    location: ObjLocation::Text {
                        fig,
                        axes: new_idx,
                        idx: t,
                    },
                    extra: BTreeMap::new(),
                },
            );
        }
        self.current_object = h;
        self.dirty = true;
        Some(h)
    }

    /// The registered kind of the series at `(fig, axes, series)` by looking up
    /// the handle whose location matches; defaults to `Line` if not registered.
    fn kind_of_series_at(&self, fig: u64, axes: usize, series: usize) -> ObjKind {
        self.objects
            .values()
            .find(|r| {
                matches!(r.location, ObjLocation::Series { fig: f, axes: a, series: s }
                    if f == fig && a == axes && s == series)
            })
            .map(|r| r.kind)
            .unwrap_or(ObjKind::Line)
    }

    /// Immutable access to a series by location, if it still exists.
    #[must_use]
    pub fn series_at(&self, loc: ObjLocation) -> Option<&Series> {
        if let ObjLocation::Series { fig, axes, series } = loc {
            self.scene.figure(fig)?.axes.get(axes)?.series.get(series)
        } else {
            None
        }
    }

    /// Immutable access to a text annotation by location, if it still exists.
    #[must_use]
    pub fn text_at(&self, loc: ObjLocation) -> Option<&fm_graphics::TextLabel> {
        if let ObjLocation::Text { fig, axes, idx } = loc {
            self.scene.figure(fig)?.axes.get(axes)?.texts.get(idx)
        } else {
            None
        }
    }

    /// Publish the current scene through the sink (if any) and clear `dirty`.
    /// This is what `drawnow` (and an implicit post-command draw) calls.
    pub fn flush(&mut self) {
        if let Some(sink) = &self.sink {
            sink.publish(&self.scene);
        }
        self.dirty = false;
    }
}

/// True if `loc` belongs to figure `fig` (the figure itself or any descendant).
fn location_in_figure(loc: ObjLocation, fig: u64) -> bool {
    match loc {
        ObjLocation::Figure { fig: f }
        | ObjLocation::Axes { fig: f, .. }
        | ObjLocation::Series { fig: f, .. }
        | ObjLocation::Text { fig: f, .. } => f == fig,
    }
}

/// True if `loc` is the axes `(fig, axes)` or one of its series children.
fn location_is_axes_or_child(loc: ObjLocation, fig: u64, axes: usize) -> bool {
    match loc {
        ObjLocation::Axes { fig: f, axes: a } => f == fig && a == axes,
        ObjLocation::Series {
            fig: f, axes: a, ..
        }
        | ObjLocation::Text {
            fig: f, axes: a, ..
        } => f == fig && a == axes,
        ObjLocation::Figure { .. } => false,
    }
}
