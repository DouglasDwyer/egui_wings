#![deny(warnings)]
#![forbid(unsafe_code)]
#![warn(clippy::missing_docs_in_private_items)]

use egui_wings::*;
use geese::*;

/// Implements the `egui_wings::Egui` trait for WASM guest modules.
pub struct EguiHost {
    /// The `egui` context to share with WASM modules.
    ctx: Context
}

impl EguiHost {
    /// Gets a reference to the shared context.
    pub fn context(&self) -> &Context {
        &self.ctx
    }

    /// Sets the context that will be shared with WASM modules.
    pub fn set_context(&mut self, ctx: Context) {
        self.ctx = ctx;
    }
}

impl AsMut<dyn Egui> for EguiHost {
    fn as_mut(&mut self) -> &mut dyn Egui {
        self
    }
}

impl Egui for EguiHost {
    fn begin_context_edit(&self, deltas: ContextSnapshotDeltas) -> CreateContextSnapshot {
        CreateContextSnapshot::FromContext(self.ctx.clone(), deltas)
    }

    fn end_context_edit(&self, state: CreateContextSnapshot) {
        let CreateContextSnapshot::Created(to_apply) = state else { unreachable!() };
        self.ctx.apply_snapshot(to_apply);
    }
}

impl GeeseSystem for EguiHost {
    fn new(_: GeeseContextHandle<Self>) -> Self {
        Self {
            ctx: Context::default()
        }
    }
}