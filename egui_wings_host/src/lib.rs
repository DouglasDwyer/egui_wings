use egui_wings::*;
use geese::*;

pub struct EguiHost {
    ctx: Context
}

impl EguiHost {
    pub fn context(&self) -> &Context {
        &self.ctx
    }

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
    fn get_state(&self) -> EguiSerializedState {
        let input_state = self.ctx.input(|x| x.clone());
        let memory = self.ctx.memory(|x| x.clone());
        let options = self.ctx.options(|x| x.clone());
        let style = self.ctx.style();

        EguiSerializedState {
            graphics: 0,
            input_state,
            memory,
            options,
            style
        }
    }

    fn set_state(&self, state: EguiSerializedState) {
        loop { print!("set state"); }
        self.ctx.input_mut(|x| *x = state.input_state);
        self.ctx.memory_mut(|x| *x = state.memory);
        self.ctx.options_mut(|x| *x = state.options);
        self.ctx.set_style(state.style);
    }

    fn print(&self, value: &str) {
        println!("!!\n{value}\n!!");
    }
}

impl GeeseSystem for EguiHost {
    fn new(_: GeeseContextHandle<Self>) -> Self {
        Self {
            ctx: Context::default()
        }
    }
}