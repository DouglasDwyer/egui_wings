use core::str;

use egui_wings::*;
use example_host::*;
use wings::*;

instantiate_systems!(ExampleHost, [PluginSystem]);

#[export_system]
pub struct PluginSystem {
    ctx: WingsContextHandle<Self>,
    text: String
}

impl PluginSystem {
    fn draw_ui(&mut self, _: &example_host::on::Render) {
        let egui = self.ctx.get::<dyn Egui>();
        let ctx = egui.context();
        for i in 0..1 {
            Window::new(format!("winit + egui + wgpu says hello! {i}"))
                .resizable(true)
                .vscroll(true)
                .default_open(false)
            .show(&ctx, |ui| {
                ui.label("Label!");
    
                if ui.button("Button!").clicked() {
                    global_print("boom!");
                }
    
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Pixels per point: {}",
                        ctx.pixels_per_point()
                    ));
                    if ui.button("-").clicked() {
                        //scale_factor = (scale_factor - 0.1).max(0.3);
                    }
                    if ui.button("+").clicked() {
                        //scale_factor = (scale_factor + 0.1).min(3.0);
                    }
                });
    
                ui.text_edit_singleline(&mut self.text);
            });
        }
    }
}

impl WingsSystem for PluginSystem {
    const DEPENDENCIES: Dependencies = dependencies().with::<dyn Egui>();

    const EVENT_HANDLERS: EventHandlers<Self> = event_handlers().with(Self::draw_ui);

    fn new(ctx: WingsContextHandle<Self>) -> Self {
        std::panic::set_hook(Box::new(|x| global_print(&format!("{x}"))));

        Self { ctx, text: String::default() }
    }
}