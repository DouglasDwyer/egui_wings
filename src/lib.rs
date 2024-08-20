#![deny(warnings)]
#![warn(clippy::missing_docs_in_private_items)]

//! # egui_wings
//! 
//! This crate facilitates sharing an `egui::Context` between a host and multiple guest WASM modules. This allows WASM plugins to draw UI and easily display it via the host.
//! 
//! ---
//! 
//! ### Usage
//! 
//! The following code snippet shows how to use `egui_wings` from a WASM plugin (the complete example may be found in the [`egui_wings_example` folder](/egui_wings_example/)). It defines a `WingsSystem` which will store the WASM plugin's state. Each frame, the `draw_ui` method is invoked. It accesses the host `egui::Context` via a system dependency and then makes normal `egui` calls to draw a UI.
//! 
//! ```ignore
//! use egui_wings::*;
//! use example_host::*;
//! use wings::*;
//! 
//! instantiate_systems!(ExampleHost, [PluginSystem]);
//! 
//! /// An object that will be instantiated inside a WASM plugin.
//! #[export_system]
//! pub struct PluginSystem {
//!     /// A handle for accessing system dependencies.
//!     ctx: WingsContextHandle<Self>,
//! }
//! 
//! impl PluginSystem {
//!     /// Submits the `egui` commands to draw the debug windows.
//!     fn draw_ui(&mut self, _: &example_host::on::Render) {
//!         let egui = self.ctx.get::<dyn Egui>();
//!         Window::new("webassembly says hello!")
//!             .resizable(true)
//!             .vscroll(true)
//!             .default_open(false)
//!         .show(&egui.context(), |ui| {
//!             ui.label("Hello there!");
//!         });
//!     }
//! }
//! 
//! impl WingsSystem for PluginSystem {
//!     const DEPENDENCIES: Dependencies = dependencies().with::<dyn Egui>();
//! 
//!     const EVENT_HANDLERS: EventHandlers<Self> = event_handlers().with(Self::draw_ui);
//! 
//!     fn new(ctx: WingsContextHandle<Self>) -> Self {
//!         Self { ctx }
//!     }
//! }
//! ```

pub use crate::snapshot::ContextSnapshotDeltas;
use crate::snapshot::*;
/// Exports the `egui` crate for easy use.
pub use egui;
use egui::*;
use serde::*;
use std::ops::*;
use std::sync::*;
use wings::*;

/// Abuses compiler behavior to get access to `egui`'s private state, so that
/// it may be serialized. Temporary hack until [`https://github.com/emilk/egui/pull/4930`]
mod private_hack;

/// Holds the serialization logic for taking snapshots.
mod snapshot;

/// The inner context which temporarily stores state that will be copied to the host.
static CONTEXT: OnceLock<Context> = OnceLock::new();

/// Allows for accessing the shared `egui::Context` for plugins.
#[system_trait(host)]
pub trait Egui: 'static {
    /// Begins a context transaction by obtaining a snapshot containing the state
    /// of the host's `egui::Context` so that it may be edited on the guest.
    #[doc(hidden)]
    fn begin_context_edit(&self, deltas: ContextSnapshotDeltas) -> CreateContextSnapshot;

    /// Updates the host `egui::Context` to use the given guest state, finishing the transaction.
    #[doc(hidden)]
    fn end_context_edit(&self, state: CreateContextSnapshot);
}

impl dyn Egui {
    /// Initiates an `egui` transaction and produces a temporary handle to the `egui::Context`.
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
            ContextSnapshotDeltas::from_context(context)
        };

        self.begin_context_edit(deltas).apply(context);
        let initial_deltas = ContextSnapshotDeltas::from_context(context);

        EguiHandle {
            ctx: self,
            initial_deltas
        }
    }
}

/// Provides access to an `egui::Context` which is synchronized with the host.
/// The `egui::Context` may be cloned, but the context is invalidated when this
/// handle is dropped.
pub struct EguiHandle<'a> {
    /// The underlying `egui` context.
    ctx: &'a dyn Egui,
    /// The state of the context at the beginning of the transaction.
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
        self.ctx.end_context_edit(CreateContextSnapshot::FromContext(self.clone(), self.initial_deltas));
    }
}

/// Allows for serializing a `ContextSnapshot` across the WASM boundary.
#[doc(hidden)]
pub enum CreateContextSnapshot {
    /// This variant is used whenever a snapshot is deserialized.
    Created(ContextSnapshot),
    /// When this object is serialized, it will use a snapshot of the provided
    /// context with the given deltas.
    FromContext(Context, ContextSnapshotDeltas)
}

impl CreateContextSnapshot {
    /// Applies the snapshot to the current context. Panics if this snapshot is
    /// not the `Created` variant.
    pub fn apply(self, context: &Context) {
        let Self::Created(value) = self else { panic!("Snapshot was not `Created` variant.") };
        let exposed = private_hack::Context::from_context(context);
        let mut ctx = exposed.0.write();
        
        let frame_nr = ctx.viewports.get(&ctx.last_viewport).map(|x| x.repaint.frame_nr).unwrap_or(u64::MAX);
        let new_frame = frame_nr != value.deltas.frame_count;
        if let Some(style) = value.style {
            ctx.memory.options.style = style;
        }

        Self::apply_memory_snapshot(&mut ctx, value.memory);
        Self::apply_options_snapshot(&mut ctx, &value.options);
        ctx.new_zoom_factor = value.new_zoom_factor;
        ctx.last_viewport = value.last_viewport;
        Self::apply_viewport_snapshots(&mut ctx, &value.deltas, value.viewports);
        ctx.memory.data.insert_temp(Id::NULL, value.deltas);
        let last_style = LastStyle(ctx.memory.options.style.clone());
        ctx.memory.data.insert_temp(Id::NULL, last_style);

        if let Some(font_definitions) = value.font_definitions {
            let to_insert =
                std::mem::replace(&mut ctx.memory.new_font_definitions, Some(font_definitions));
            Self::update_fonts_mut(&mut ctx);
            ctx.memory.new_font_definitions = to_insert;
        } else if new_frame {
            // Reset font cache and galleys for new frame
            let to_insert = std::mem::take(&mut ctx.memory.new_font_definitions);
            Self::update_fonts_mut(&mut ctx);
            ctx.memory.new_font_definitions = to_insert;
        }
    }
    
    /// Updates the memory from the snapshot.
    fn apply_memory_snapshot(ctx: &mut private_hack::ContextImpl, snapshot: MemorySnapshot) {
        ctx.memory.data.insert_temp(Id::new(ViewportId::ROOT), egui::text_selection::LabelSelectionState::from(snapshot.label_selection_state));
        ctx.memory.new_font_definitions = snapshot.new_font_definitions;
        ctx.memory.viewport_id = snapshot.viewport_id;
        ctx.memory.popup = snapshot.popup;
        ctx.memory.everything_is_visible = snapshot.everything_is_visible;
        ctx.memory.layer_transforms = snapshot.layer_transforms;
        ctx.memory.areas = snapshot.areas;
        ctx.memory.interactions = snapshot.interactions;
        ctx.memory.focus = snapshot.focus;
    }

    /// Updates the options from the snapshot.
    fn apply_options_snapshot(ctx: &mut private_hack::ContextImpl, snapshot: &OptionsSnapshot) {
        ctx.memory.options.zoom_factor = snapshot.zoom_factor;
        ctx.memory.options.zoom_with_keyboard = snapshot.zoom_with_keyboard;
        ctx.memory.options.tessellation_options = snapshot.tessellation_options;
        ctx.memory.options.repaint_on_widget_change = snapshot.repaint_on_widget_change;
        ctx.memory.options.screen_reader = snapshot.screen_reader;
        ctx.memory.options.preload_font_glyphs = snapshot.preload_font_glyphs;
        ctx.memory.options.warn_on_id_clash = snapshot.warn_on_id_clash;
        ctx.memory.options.line_scroll_speed = snapshot.line_scroll_speed;
        ctx.memory.options.scroll_zoom_speed = snapshot.scroll_zoom_speed;
        ctx.memory.options.reduce_texture_memory = snapshot.reduce_texture_memory;
    }

    /// Updates the list of viewports from the snapshot list.
    fn apply_viewport_snapshots(ctx: &mut private_hack::ContextImpl, deltas: &ContextSnapshotDeltas, snapshots: ViewportIdMap<ViewportStateSnapshot>) {
        ctx.viewports.retain(|x, _| snapshots.contains_key(x));
        for (id, snapshot) in snapshots {
            let viewport = ctx
                .viewports
                .get_mut(&id)
                .expect("Failed to get viewport.");
            viewport.class = snapshot.class;
            viewport.builder = snapshot.builder;
            viewport.input = snapshot.input;
            viewport.this_frame = snapshot.this_frame;
            viewport.prev_frame = snapshot.prev_frame;
            viewport.used = snapshot.used;
            viewport.hits = snapshot.hits;
            viewport.interact_widgets = snapshot.interact_widgets;
            viewport.repaint.frame_nr = deltas.frame_count;
            viewport.graphics = snapshot.graphics;
            viewport.output = snapshot.output;
            viewport.commands = snapshot.commands;
        }
        
        Self::reinitialize_galleys(ctx)
    }

    /// Reloads all galleys from the cache, because galley data is not serialized
    /// within [`ContextSnapshot`]s.
    fn reinitialize_galleys(ctx: &mut private_hack::ContextImpl) {
        let pixels_per_point = ctx.viewports.get(&ctx.last_viewport).map(|x| x.input.pixels_per_point).unwrap_or(1.0);
        if let Some(fonts) = ctx.fonts.get(&pixels_per_point.into()) {
            for viewport in ctx.viewports.values_mut() {
                for paint_lists in viewport.graphics.as_inner_mut() {
                    for paint_list in paint_lists.values_mut() {
                        for clipped_shape in paint_list.as_inner_mut() {
                            Self::reinitialize_galleys_for_shape(&mut clipped_shape.shape, fonts);
                        }
                    }
                }
            }
        }
    }

    /// Reinitializes any galleys associated with this shape from the cache,
    /// because galley data is not serialized within [`ContextSnapshot`]s.
    fn reinitialize_galleys_for_shape(shape: &mut Shape, fonts: &egui::epaint::Fonts) {
        match shape {
            Shape::Vec(x) => {
                for shape in x {
                    Self::reinitialize_galleys_for_shape(shape, fonts);
                }
            }
            Shape::Text(x) => x.galley = fonts.layout_job((*x.galley.job).clone()),
            _ => {}
        }
    }

    /// Reloads the font definitions.
    fn update_fonts_mut(ctx: &mut private_hack::ContextImpl) {
        if let Some(viewport) = ctx.viewports.get(&ctx.last_viewport) {
            let input = &viewport.input;
            let pixels_per_point = input.pixels_per_point();
            let max_texture_side = input.max_texture_side;
    
            if let Some(font_definitions) = ctx.memory.new_font_definitions.take() {
                // New font definition loaded, so we need to reload all fonts.
                ctx.fonts.clear();
                ctx.font_definitions = font_definitions;
            }
    
            let mut is_new = false;
    
            let fonts = ctx
                .fonts
                .entry(pixels_per_point.into())
                .or_insert_with(|| {
                    is_new = true;
                    egui::epaint::Fonts::new(
                        pixels_per_point,
                        max_texture_side,
                        ctx.font_definitions.clone(),
                    )
                });
    
            {
                fonts.begin_frame(pixels_per_point, max_texture_side);
            }
    
            if is_new && ctx.memory.options.preload_font_glyphs {
                // Preload the most common characters for the most common fonts.
                // This is not very important to do, but may save a few GPU operations.
                for font_id in ctx.memory.options.style.text_styles.values() {
                    fonts.lock().fonts.font(font_id).preload_common_characters();
                }
            }
        }
    }
}

impl Serialize for CreateContextSnapshot {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            CreateContextSnapshot::FromContext(context, deltas) => {
                let current_deltas = ContextSnapshotDeltas::from_context(context);
                let exposed = private_hack::Context::from_context(context);
                let ctx = exposed.0.read();

                let style = (deltas.style_count != current_deltas.style_count)
                    .then(|| ctx.memory.options.style.clone());
    
                let font_definitions = (deltas.font_definitions_count
                    != current_deltas.font_definitions_count)
                    .then_some(&ctx.font_definitions);
    
                let borrow = ContextShapshotBorrow {
                    deltas: &current_deltas,
                    font_definitions,
                    memory: &ctx.memory,
                    style,
                    new_zoom_factor: &ctx.new_zoom_factor,
                    last_viewport: &ctx.last_viewport,
                    viewports: &ctx.viewports,
                };
                <ContextShapshotBorrow as Serialize>::serialize(&borrow, serializer)
            },
            CreateContextSnapshot::Created(_) => Err(serde::ser::Error::custom("Cannot serialize created snapshot")),
        }
    }
}

impl<'de> Deserialize<'de> for CreateContextSnapshot {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::Created(<ContextSnapshot as Deserialize>::deserialize(deserializer)?))
    }
}

/// Tracks the last style that was applied.
#[derive(Clone)]
struct LastStyle(Arc<private_hack::Style>);