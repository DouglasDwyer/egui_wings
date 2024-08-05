use egui_wings::*;
use example_host::*;
use wings::*;

instantiate_systems!(ExampleHost, [PluginSystem]);

#[export_system]
pub struct PluginSystem {
    ctx: WingsContextHandle<Self>,
    click_count: u32,
    text: String
}

impl PluginSystem {
    fn draw_ui(&mut self, _: &example_host::on::Render) {
        let egui = self.ctx.get::<dyn Egui>();
        let ctx = egui.context();
        Window::new(format!("webassemcbly says hello!"))
            .resizable(true)
            .vscroll(true)
            .default_open(false)
        .show(&ctx, |ui| {
            ui.label(format!("Click count: {}", self.click_count));

            if ui.button("Button!").clicked() {
                self.click_count += 1;
            }

            ui.separator();    
            ui.text_edit_singleline(&mut self.text);
        });
    }
}

impl WingsSystem for PluginSystem {
    const DEPENDENCIES: Dependencies = dependencies().with::<dyn Egui>();

    const EVENT_HANDLERS: EventHandlers<Self> = event_handlers().with(Self::draw_ui);

    fn new(ctx: WingsContextHandle<Self>) -> Self {
        Self { ctx, click_count: 0, text: String::default() }
    }
}