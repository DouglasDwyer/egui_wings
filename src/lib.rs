pub use egui::*;
use egui::layers::*;
use std::mem::*;
use std::ops::*;
use std::sync::*;
use wings::*;

static CONTEXT: OnceLock<Context> = OnceLock::new();

#[system_trait(host)]
pub trait Egui: 'static {
    fn get_state(&self) -> EguiSerializedState;
    fn set_state(&self, state: EguiSerializedState);

    #[global(global_print)]
    fn print(&self, value: &str);
}

impl dyn Egui {
    pub fn context(&self) -> EguiHandle {
        let context = CONTEXT.get_or_init(Context::default);
        let state = self.get_state();
        context.input_mut(|x| *x = state.input_state);
        context.memory_mut(|x| *x = state.memory);
        context.options_mut(|x| *x = state.options);
        context.set_style(state.style);

        EguiHandle {
            ctx: self
        }
    }
}

#[export_type]
pub struct EguiSerializedState {
    pub graphics: u32,
    pub input_state: InputState,
    pub memory: Memory,
    pub options: Options,
    pub style: Arc<Style>
}

pub struct EguiHandle<'a> {
    ctx: &'a dyn Egui
}

impl<'a> Deref for EguiHandle<'a> {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        CONTEXT.get().expect("Failed to get egui context.")
    }
}

impl<'a> Drop for EguiHandle<'a> {
    fn drop(&mut self) {
        let graphics = self.graphics(Clone::clone);
        let input_state = self.input(Clone::clone);
        let memory = self.memory(Clone::clone);
        let options = self.options(Clone::clone);
        self.ctx.set_state(EguiSerializedState {
            graphics: 0,
            input_state,
            memory,
            options,
            style: self.style()
        });
    }
}

pub struct SerializedGraphicsLayers([IdMap<PaintList>; 6]);

impl SerializedGraphicsLayers {
    pub fn len(&self) -> usize {
        self.0.iter().fold(0, |acc, x| acc + x.len())
    }
}

impl From<SerializedGraphicsLayers> for GraphicLayers {
    fn from(value: SerializedGraphicsLayers) -> Self {
        unsafe {
            transmute(value)
        }
    }
}

impl From<GraphicLayers> for SerializedGraphicsLayers {
    fn from(value: GraphicLayers) -> Self {
        unsafe {
            transmute(value)
        }
    }
}

impl From<&SerializedGraphicsLayers> for &GraphicLayers {
    fn from(value: &SerializedGraphicsLayers) -> Self {
        unsafe {
            transmute(value)
        }
    }
}

impl From<&GraphicLayers> for &SerializedGraphicsLayers {
    fn from(value: &GraphicLayers) -> Self {
        unsafe {
            transmute(value)
        }
    }
}