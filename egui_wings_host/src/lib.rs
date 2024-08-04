use egui_wings::*;
use egui_wings::marshal::*;
use geese::*;
use std::sync::*;
use std::sync::atomic::*;

pub struct EguiHost {
    ctx: Context,
    style_id: AtomicU64,
    previous_style: AtomicUsize
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
    fn get_state(&self, previous_style: u64) -> SerializedViewport {
        let style = self.ctx.style();
        let style_ptr = Arc::as_ptr(&style) as usize;
        let previous = self.previous_style.swap(style_ptr, Ordering::Relaxed);
        
        let mut style_id = self.style_id.load(Ordering::Relaxed);
        let mut serialize_style = style_ptr != previous || style_id != previous_style;

        if serialize_style {
            style_id = self.style_id.fetch_add(1, Ordering::Relaxed) + 1
        }

        SerializedViewport::FromContext {
            context: self.ctx.clone(),
            serialize_style,
            style_id
        }
    }

    fn set_state(&self, state: SerializedViewport) {
        let SerializedViewport::Owned { viewport } = state else { panic!("Received context.") };
        
        self.style_id.store(viewport.style_id, Ordering::Relaxed);
        self.previous_style.store(Arc::as_ptr(&self.ctx.style()) as usize, Ordering::Relaxed);
        viewport.apply_to_context(&self.ctx);
    }

    fn print(&self, value: &str) {
        println!("!!\n{value}\n!!");
    }
}

impl GeeseSystem for EguiHost {
    fn new(_: GeeseContextHandle<Self>) -> Self {
        Self {
            ctx: Context::default(),
            style_id: AtomicU64::new(0),
            previous_style: AtomicUsize::new(0)
        }
    }
}