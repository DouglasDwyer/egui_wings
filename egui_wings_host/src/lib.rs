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
    fn get_snapshot(&self, deltas: ContextSnapshotDeltas) -> CreateContextSnapshot {
        /*self.ctx.snapshot_for(&ContextSnapshotDeltas::default(), |x| {
            let serded = egui_wings::wings::marshal::bincode::serialize(x).unwrap();
            let unserded: ContextSnapshot = egui_wings::wings::marshal::bincode::deserialize(&serded).unwrap();
            println!("SUCC ess");
        });*/
        CreateContextSnapshot::FromContext(self.ctx.clone(), deltas)
    }

    fn set_snapshot(&self, state: CreateContextSnapshot) {
        let CreateContextSnapshot::Created(to_apply) = state else { unreachable!() };
        self.ctx.apply_snapshot(to_apply);
        //self.ctx.graphics(|x| println!("lcl: {}", x.print_it()));
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