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
    fn get_state(&self) -> SerializedViewport {
        SerializedViewport::FromContext(self.ctx.clone())
    }

    fn set_state(&self, state: SerializedViewport) {
        let SerializedViewport::Owned(owned_state) = state else { panic!("Received context.") };
        owned_state.apply_to_context(&self.ctx);
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