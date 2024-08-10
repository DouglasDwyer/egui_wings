use egui_demo_lib::*;
use egui_wings::*;
use egui_wings::egui::*;
use example_host::*;
use wings::*;

instantiate_systems!(ExampleHost, [PluginSystem]);

/// An object that will be instantiated inside a WASM plugin.
#[export_system]
pub struct PluginSystem {
    /// A handle for accessing system dependencies.
    ctx: WingsContextHandle<Self>,
    /// The number of times the example button was clicked.
    click_count: u32,
    /// The text that the user entered in the example field.
    text: String,
    /// The widget gallery window.
    gallery: WidgetGallery
}

impl PluginSystem {
    /// Submits the `egui` commands to draw the debug windows.
    fn draw_ui(&mut self, _: &example_host::on::Render) {
        let egui = self.ctx.get::<dyn Egui>();
        let ctx = egui.context();

        Window::new("webassembly says hello!")
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

        self.gallery.show(&ctx, &mut true);
    }
}

impl WingsSystem for PluginSystem {
    const DEPENDENCIES: Dependencies = dependencies().with::<dyn Egui>();

    const EVENT_HANDLERS: EventHandlers<Self> = event_handlers().with(Self::draw_ui);

    fn new(ctx: WingsContextHandle<Self>) -> Self {
        Self { ctx, click_count: 0, text: String::default(), gallery: WidgetGallery::default() }
    }
}