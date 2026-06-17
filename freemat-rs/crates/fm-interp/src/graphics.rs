//! Interpreter-held graphics state: the retained [`Scene`], the current figure
//! id, and an optional [`GraphicsSink`] the webserver installs to broadcast
//! scene updates.
//!
//! The sink is **optional** — when none is installed (library tests, the
//! conformance harness), graphics builtins still mutate the retained scene; they
//! just don't publish. So nothing here pulls in `axum`/`tokio`.

use fm_graphics::{Figure, GraphicsSink, Scene};

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
}

impl GraphicsState {
    /// Ensure there is a current figure, creating figure 1 if none exists, and
    /// return its id.
    pub fn ensure_figure(&mut self) -> u64 {
        if self.current_figure == 0 {
            self.current_figure = 1;
            self.scene.figure_mut_or_insert(1);
        } else {
            self.scene.figure_mut_or_insert(self.current_figure);
        }
        self.current_figure
    }

    /// Select (or create) a figure by id and make it current.
    pub fn select_figure(&mut self, id: u64) {
        self.current_figure = id;
        self.scene.figure_mut_or_insert(id);
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

    /// Publish the current scene through the sink (if any) and clear `dirty`.
    /// This is what `drawnow` (and an implicit post-command draw) calls.
    pub fn flush(&mut self) {
        if let Some(sink) = &self.sink {
            sink.publish(&self.scene);
        }
        self.dirty = false;
    }
}
