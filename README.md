# egui_wings

[![Crates.io](https://img.shields.io/crates/v/egui_wings.svg)](https://crates.io/crates/egui_wings)
[![Docs.rs](https://docs.rs/egui_wings/badge.svg)](https://docs.rs/egui_wings)

This crate facilitates sharing an `egui::Context` between a host and multiple guest WASM modules. This allows WASM plugins to draw UI and easily display it via the host.

---

### Usage

The following code snippet shows how to use `egui_wings` from a WASM plugin (the complete example may be found in the [`egui_wings_example` folder](/egui_wings_example/)). It defines a `WingsSystem` which will store the WASM plugin's state. Each frame, the `draw_ui` method is invoked. It accesses the host `egui::Context` via a system dependency and then makes normal `egui` calls to draw a UI.

```rust
use egui_wings::*;
use example_host::*;
use wings::*;

instantiate_systems!(ExampleHost, [PluginSystem]);

/// An object that will be instantiated inside a WASM plugin.
#[export_system]
pub struct PluginSystem {
    /// A handle for accessing system dependencies.
    ctx: WingsContextHandle<Self>,
}

impl PluginSystem {
    /// Submits the `egui` commands to draw the debug windows.
    fn draw_ui(&mut self, _: &example_host::on::Render) {
        let egui = self.ctx.get::<dyn Egui>();
        Window::new("webassembly says hello!")
            .resizable(true)
            .vscroll(true)
            .default_open(false)
        .show(&egui.context(), |ui| {
            ui.label("Hello there!");
        });
    }
}

impl WingsSystem for PluginSystem {
    const DEPENDENCIES: Dependencies = dependencies().with::<dyn Egui>();

    const EVENT_HANDLERS: EventHandlers<Self> = event_handlers().with(Self::draw_ui);

    fn new(ctx: WingsContextHandle<Self>) -> Self {
        Self { ctx }
    }
}
```