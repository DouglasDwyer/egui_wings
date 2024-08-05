use crate::marshal::*;
pub use egui::*;
use serde::*;
use std::mem::*;
use std::ops::*;
use std::sync::*;
use wings::*;
pub use wings;

static CONTEXT: OnceLock<Context> = OnceLock::new();

#[system_trait(host)]
pub trait Egui: 'static {
    fn get_snapshot(&self, deltas: ContextSnapshotDeltas) -> CreateContextSnapshot;
    fn set_snapshot(&self, state: CreateContextSnapshot);

    #[global(global_print)]
    fn print(&self, value: &str);
    #[global(global_time)]
    fn hal_time(&self) -> u128;
}

impl dyn Egui {
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
        let startt = global_time();
        let CreateContextSnapshot::Created(snapshot) = self.get_snapshot(deltas) else { unreachable!() };
        let endd = global_time() - startt;
        
        context.apply_snapshot(snapshot);

        context.snapshot_for(&deltas, |x| {
            let serded = wings::marshal::bincode::serialize(x).unwrap();
            let startt2 = global_time();
            let edded = wings::marshal::bincode::deserialize::<ContextSnapshot>(&serded).unwrap();
            let endd2 = global_time() - startt2;

            global_print(&format!("The initial snappy was {} bytes and deser in {endd2}, but it took {endd}", serded.len()));
        });

        let initial_deltas = context.snapshot_deltas();

        EguiHandle {
            ctx: self,
            initial_deltas
        }
    }
}
    
pub struct EguiHandle<'a> {
    ctx: &'a dyn Egui,
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
        self.ctx.set_snapshot(CreateContextSnapshot::FromContext(self.clone(), self.initial_deltas));
    }
}

pub enum CreateContextSnapshot {
    Created(ContextSnapshot),
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