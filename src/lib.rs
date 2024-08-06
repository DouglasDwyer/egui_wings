pub use egui::*;
use serde::*;
use std::ops::*;
use std::sync::*;
use wings::*;

/// The inner context which temporarily stores state that will be copied to the host.
static CONTEXT: OnceLock<Context> = OnceLock::new();

/// Allows for accessing the shared `egui::Context` for plugins.
#[system_trait(host)]
pub trait Egui: 'static {
    /// Begins a context transaction by obtaining a snapshot containing the state
    /// of the host's `egui::Context` so that it may be edited on the guest.
    #[doc(hidden)]
    fn begin_context_edit(&self, deltas: ContextSnapshotDeltas) -> CreateContextSnapshot;

    /// Updates the host `egui::Context` to use the given guest state, finishing the transaction.
    #[doc(hidden)]
    fn end_context_edit(&self, state: CreateContextSnapshot);
}

impl dyn Egui {
    /// Initiates an `egui` transaction and produces a temporary handle to the `egui::Context`.
    pub fn context(&self) -> EguiHandle {
        let mut initialized = false;
        let context = CONTEXT.get_or_init(|| {
            let result = Context::default();
            result.begin_frame(RawInput::default());
            initialized = true;
            result
        });
        
        let deltas = if initialized {
            ContextSnapshotDeltas::default()
        }
        else {
            context.snapshot_deltas()
        };

        let CreateContextSnapshot::Created(snapshot) = self.begin_context_edit(deltas) else { unreachable!() };
        context.apply_snapshot(snapshot);
        let initial_deltas = context.snapshot_deltas();

        EguiHandle {
            ctx: self,
            initial_deltas
        }
    }
}

/// Provides access to an `egui::Context` which is synchronized with the host.
/// The `egui::Context` may be cloned, but the context is invalidated when this
/// handle is dropped.
pub struct EguiHandle<'a> {
    /// The underlying `egui` context.
    ctx: &'a dyn Egui,
    /// The state of the context at the beginning of the transaction.
    initial_deltas: ContextSnapshotDeltas
}

impl<'a> Deref for EguiHandle<'a> {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        CONTEXT.get().expect("Failed to get egui context.")
    }
}

impl<'a> Drop for EguiHandle<'a> {
    fn drop(&mut self) {
        self.ctx.end_context_edit(CreateContextSnapshot::FromContext(self.clone(), self.initial_deltas));
    }
}

/// Allows for serializing a `ContextSnapshot` across the WASM boundary.
#[doc(hidden)]
pub enum CreateContextSnapshot {
    /// This variant is used whenever a snapshot is deserialized.
    Created(ContextSnapshot),
    /// When this object is serialized, it will use a snapshot of the provided
    /// context with the given deltas.
    FromContext(Context, ContextSnapshotDeltas)
}

impl Serialize for CreateContextSnapshot {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            CreateContextSnapshot::FromContext(ctx, deltas) => ctx.snapshot_for(deltas, |x| Serialize::serialize(x, serializer)),
            CreateContextSnapshot::Created(_) => Err(serde::ser::Error::custom("Cannot serialize created snapshot")),
        }
    }
}

impl<'de> Deserialize<'de> for CreateContextSnapshot {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::Created(<ContextSnapshot as Deserialize>::deserialize(deserializer)?))
    }
}