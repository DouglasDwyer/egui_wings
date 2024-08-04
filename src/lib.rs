use crate::marshal::*;
pub use egui::*;
use egui::epaint::*;
use egui::layers::*;
use egui::style::*;
use ser::SerializeSeq;
use ser::SerializeStruct;
use serde::*;
use std::collections::*;
use std::mem::*;
use std::ops::*;
use std::sync::*;
use std::sync::atomic::*;
use wings::*;

mod arc_cache;

static CONTEXT: OnceLock<Context> = OnceLock::new();
static STYLE_ID: AtomicU64 = AtomicU64::new(u64::MAX);

#[system_trait(host)]
pub trait Egui: 'static {
    fn get_state(&self, style_id: u64) -> SerializedViewport;
    fn set_state(&self, state: SerializedViewport);

    #[global(global_print)]
    fn print(&self, value: &str);
}

impl dyn Egui {
    pub fn context(&self) -> EguiHandle {
        let context = CONTEXT.get_or_init(|| {
            let result = Context::default();
            result.begin_frame(RawInput::default());
            result
        });
        
        let SerializedViewport::Owned { viewport } = self.get_state(STYLE_ID.load(Ordering::Relaxed)) else { panic!("Received context.") };
        
        STYLE_ID.store(viewport.style_id, Ordering::Relaxed);
        viewport.apply_to_context(context);
        let previous_style = context.style();

        EguiHandle {
            ctx: self,
            previous_style
        }
    }
}
    
pub struct EguiHandle<'a> {
    ctx: &'a dyn Egui,
    previous_style: Arc<Style>
}

impl<'a> Deref for EguiHandle<'a> {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        CONTEXT.get().expect("Failed to get egui context.")
    }
}

impl<'a> Drop for EguiHandle<'a> {
    fn drop(&mut self) {
        let style = self.style();
        let serialize_style = !Arc::ptr_eq(&style, &self.previous_style);

        let s_memory = unsafe { self.memory(|x| wings::marshal::bincode::serialized_size(transmute::<_, &SerializedMemory>(x)).unwrap()) };
        let mut s_graphics = 0;
        let mut g_data = vec!();

        self.viewport(|x| {
            s_graphics = wings::marshal::bincode::serialized_size(<_ as AsRef<SerializedGraphicsLayers>>::as_ref(&x.graphics)).unwrap();
            let paint_lists: &[IdMap<PaintList>; 6] = x.graphics.as_ref();
            for list in paint_lists {
                for (_, vv) in list {
                    let listt: &Vec<ClippedShape> = vv.as_ref();
                    g_data.extend(listt.clone());
                }
            }
        });

        global_print(&format!("SIZES | graphics: {s_graphics} | memory: {s_memory} || \n{g_data:?}"));

        let style_id = if serialize_style {
            STYLE_ID.fetch_add(1, Ordering::Relaxed) + 1
        }
        else {
            STYLE_ID.load(Ordering::Relaxed)
        };

        self.ctx.set_state(SerializedViewport::FromContext { context: self.clone(), serialize_style, style_id });
    }
}

pub mod marshal {
    use super::*;

    pub enum SerializedViewport {
        FromContext {
            context: Context,
            serialize_style: bool,
            style_id: u64
        },
        Owned {
            viewport: SerializedViewportOwned
        }
    }
    
    impl Serialize for SerializedViewport {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            match self {
                SerializedViewport::FromContext { context, serialize_style, style_id } => {
                    let mut sequence = serializer.serialize_struct("SerializedViewportOwned", 10)?;
                    
                    unsafe { context.memory(|x| sequence.serialize_field("memory", transmute::<_, &SerializedMemory>(x)))? };
    
                    sequence.serialize_field("style", &serialize_style.then(|| context.style()));
                    sequence.serialize_field("style_id", style_id);
                    
                    context.viewport(|x| {
                        sequence.serialize_field("graphics", <_ as AsRef<SerializedGraphicsLayers>>::as_ref(&x.graphics))?;
                        sequence.serialize_field("input_state", &x.input)?;
                        sequence.serialize_field("this_frame", &x.this_frame)?;
                        sequence.serialize_field("prev_frame", &x.prev_frame)?;
                        sequence.serialize_field("used", &x.used)?;
                        sequence.serialize_field("hits", &x.hits)?;
                        sequence.serialize_field("interact_widgets", &x.interact_widgets)?;
                        sequence.serialize_field("output", &x.output)?;
                        Ok(())
                    })?;
                    
                    sequence.end()
                },
                SerializedViewport::Owned { viewport } => <SerializedViewportOwned as Serialize>::serialize(viewport, serializer),
            }
        }
    }
    
    impl<'de> Deserialize<'de> for SerializedViewport {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            Ok(Self::Owned { viewport: <SerializedViewportOwned as Deserialize>::deserialize(deserializer)? })
        }
    }
    
    #[derive(Serialize, Deserialize)]
    pub struct SerializedViewportOwned {
        pub memory: SerializedMemory,
        pub style: Option<Arc<Style>>,
        pub style_id: u64,
        pub graphics: SerializedGraphicsLayers,
        pub input_state: InputState,
        pub this_frame: egui::frame_state::FrameState,
        pub prev_frame: egui::frame_state::FrameState,
        pub used: bool,
        pub hits: egui::hit_test::WidgetHits,
        pub interact_widgets: egui::interaction::InteractionSnapshot,
        pub output: PlatformOutput,
    }
    
    impl SerializedViewportOwned {
        pub fn apply_to_context(self, ctx: &Context) {
            ctx.memory_mut(|x| *x = self.memory.into());

            // todo: style will get reset anyway by the deserialization of Option! What to do
            // -- can we lazycell a default style to prevent duping?
            if let Some(style) = self.style {
                ctx.set_style(style);
            }

            ctx.viewport_mut(|x| {
                x.graphics = self.graphics.0;
                x.input = self.input_state;
                x.this_frame = self.this_frame;
                x.prev_frame = self.prev_frame;
                x.used = self.used;
                x.hits = self.hits;
                x.interact_widgets = self.interact_widgets;
                x.output = self.output;
            });
        }
    }
    
    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct SerializedMemory {
        pub options: Options,
        pub data: egui::util::IdTypeMap,
        #[serde(skip)]
        pub caches: egui::util::cache::CacheStorage,
        #[serde(skip)]
        pub new_font_definitions: Option<epaint::text::FontDefinitions>,
        pub viewport_id: ViewportId,
        pub popup: Option<Id>,
        pub everything_is_visible: bool,
        pub layer_transforms: egui::ahash::HashMap<LayerId, egui::emath::TSTransform>,
        pub areas: ViewportIdMap<SerializedAreas>,
        pub interactions: ViewportIdMap<InteractionState>,
        pub focus: ViewportIdMap<Focus>,
    }
    
    impl From<Memory> for SerializedMemory {
        fn from(value: Memory) -> Self {
            unsafe {
                transmute(value)
            }
        }
    }
    
    impl From<SerializedMemory> for Memory {
        fn from(value: SerializedMemory) -> Self {
            unsafe {
                transmute(value)
            }
        }
    }
    
    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct SerializedAreas {
        pub areas: IdMap<SerializedAreaState>,
    
        /// Back-to-front. Top is last.
        pub order: Vec<LayerId>,
    
        pub visible_last_frame: ahash::HashSet<LayerId>,
        pub visible_current_frame: ahash::HashSet<LayerId>,
    
        /// When an area want to be on top, it is put in here.
        /// At the end of the frame, this is used to reorder the layers.
        /// This means if several layers want to be on top, they will keep their relative order.
        /// So if you close three windows and then reopen them all in one frame,
        /// they will all be sent to the top, but keep their previous internal order.
        pub wants_to_be_on_top: ahash::HashSet<LayerId>,
    
        /// List of sublayers for each layer
        ///
        /// When a layer has sublayers, they are moved directly above it in the ordering.
        pub sublayers: ahash::HashMap<LayerId, HashSet<LayerId>>,
    }
    
    #[derive(Copy, Clone, Debug, Serialize, Deserialize)]
    pub struct SerializedAreaState {
        /// Last known position of the pivot.
        pub pivot_pos: Option<Pos2>,
    
        /// The anchor point of the area, i.e. where on the area the [`Self::pivot_pos`] refers to.
        pub pivot: Align2,
    
        /// Last known size.
        ///
        /// Area size is intentionally NOT persisted between sessions,
        /// so that a bad tooltip or menu size won't be remembered forever.
        /// A resizable [`Window`] remembers the size the user picked using
        /// the state in the [`Resize`] container.
        pub size: Option<Vec2>,
    
        /// If false, clicks goes straight through to what is behind us. Useful for tooltips etc.
        pub interactable: bool,
    
        /// At what time was this area first shown?
        ///
        /// Used to fade in the area.
        pub last_became_visible_at: Option<f64>,
    }
    
    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct InteractionState {
        pub potential_click_id: Option<Id>,
        pub potential_drag_id: Option<Id>,
    }
    
    #[derive(Clone, Copy, Debug, Serialize, Deserialize)]
    pub struct SerializedEventFilter {
        /// If `true`, pressing tab will act on the widget,
        /// and NOT move focus away from the focused widget.
        ///
        /// Default: `false`
        pub tab: bool,
    
        /// If `true`, pressing horizontal arrows will act on the
        /// widget, and NOT move focus away from the focused widget.
        ///
        /// Default: `false`
        pub horizontal_arrows: bool,
    
        /// If `true`, pressing vertical arrows will act on the
        /// widget, and NOT move focus away from the focused widget.
        ///
        /// Default: `false`
        pub vertical_arrows: bool,
    
        /// If `true`, pressing escape will act on the widget,
        /// and NOT surrender focus from the focused widget.
        ///
        /// Default: `false`
        pub escape: bool,
    }
    
    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct Focus {
        /// The widget with keyboard focus (i.e. a text input field).
        pub focused_widget: Option<FocusWidget>,
        /// What had keyboard focus previous frame?
        pub id_previous_frame: Option<Id>,
        /// Give focus to this widget next frame
        pub id_next_frame: Option<Id>,
        #[cfg(feature = "egui/accesskit")]
        pub id_requested_by_accesskit: Option<accesskit::NodeId>,
        /// If set, the next widget that is interested in focus will automatically get it.
        /// Probably because the user pressed Tab.
        pub give_to_next: bool,
        /// The last widget interested in focus.
        pub last_interested: Option<Id>,
        /// Set when looking for widget with navigational keys like arrows, tab, shift+tab
        pub focus_direction: FocusDirection,
        /// A cache of widget ids that are interested in focus with their corresponding rectangles.
        pub focus_widgets_cache: IdMap<Rect>,
    }
    
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    pub enum FocusDirection {
        /// Select the widget closest above the current focused widget.
        Up,
        /// Select the widget to the right of the current focused widget.
        Right,
        /// Select the widget below the current focused widget.
        Down,
        /// Select the widget to the left of the current focused widget.
        Left,
        /// Select the previous widget that had focus.
        Previous,
        /// Select the next widget that wants focus.
        Next,
        /// Don't change focus.
        #[default]
        None,
    }
    
    /// The widget with focus.
    #[derive(Clone, Copy, Debug, Serialize, Deserialize)]
    pub struct FocusWidget {
        pub id: Id,
        pub filter: SerializedEventFilter,
    }
    
    #[repr(transparent)]
    pub struct SerializedGraphicsLayers(pub GraphicLayers);
    
    impl Serialize for SerializedGraphicsLayers {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            unsafe {
                let paint_lists: &[IdMap<PaintList>; Order::COUNT] = self.0.as_ref();
                let serialized_paint_lists = transmute::<_, &[IdMap<SerializedPaintList>; Order::COUNT]>(paint_lists);
                <[IdMap<SerializedPaintList>; Order::COUNT] as Serialize>::serialize(serialized_paint_lists, serializer)
            }
        }
    }
    
    impl<'a> Deserialize<'a> for SerializedGraphicsLayers {
        fn deserialize<D: Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
            unsafe {
                let raw_paint_lists = <[IdMap<SerializedPaintList>; Order::COUNT] as Deserialize>::deserialize(deserializer)?;
                let paint_lists = transmute::<_, [IdMap<PaintList>; Order::COUNT]>(raw_paint_lists);
                Ok(Self(paint_lists.into()))
            }
        }
    }
    
    impl AsRef<SerializedGraphicsLayers> for GraphicLayers {
        fn as_ref(&self) -> &SerializedGraphicsLayers {
            unsafe {
                transmute(self)
            }
        }
    }
    
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    #[serde(remote = "Shape")]
    pub enum SerializedShape {
        Noop,
        #[serde(serialize_with = "serialize_shape_vec", deserialize_with = "deserialize_shape_vec")]
        Vec(Vec<Shape>),
        Circle(CircleShape),
        Ellipse(EllipseShape),
        LineSegment {
            points: [Pos2; 2],
            stroke: PathStroke,
        },
        Path(PathShape),
        Rect(RectShape),
        // todo: text contains galley(s) which is info that we should not be serializing every frame!
        // use a cache to only serialize font data when it changes
        Text(TextShape),
        Mesh(Mesh),
        QuadraticBezier(QuadraticBezierShape),
        CubicBezier(CubicBezierShape),
        #[serde(skip_serializing, skip_deserializing)]
        Callback(PaintCallback),
    }
    
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct SerializedClippedShapeHelper {
        pub clip_rect: emath::Rect,
        #[serde(with = "SerializedShape")]
        pub shape: Shape,
    }
    
    #[derive(Clone, Debug, PartialEq)]
    #[repr(transparent)]
    struct SerializedClippedShape(ClippedShape);
    
    impl Serialize for SerializedClippedShape {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            unsafe {
                let mut struct_serializer = serializer.serialize_struct("ClippedShape", 2)?;
                struct_serializer.serialize_field("clip_rect", &self.0.clip_rect)?;
                struct_serializer.serialize_field("shape", transmute::<_, &SerializedShapeHelper>(&self.0.shape))?;
                struct_serializer.end()
            }
        }
    }
    
    impl<'de> Deserialize<'de> for SerializedClippedShape {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let helper = <SerializedClippedShapeHelper as Deserialize>::deserialize(deserializer)?;
            Ok(Self(ClippedShape {
                clip_rect: helper.clip_rect,
                shape: helper.shape
            }))
        }
    }
    
    #[repr(transparent)]
    struct SerializedPaintList(PaintList);
    
    impl Serialize for SerializedPaintList {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            unsafe {
                let paint_list: &Vec<ClippedShape> = self.0.as_ref();
                let serialized_paint_list = transmute::<_, &Vec<SerializedClippedShape>>(paint_list);
                <Vec<SerializedClippedShape> as Serialize>::serialize(serialized_paint_list, serializer)
            }
        }
    }
    
    impl<'de> Deserialize<'de> for SerializedPaintList {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            unsafe {
                let raw_paint_list = <Vec<SerializedClippedShape> as Deserialize>::deserialize(deserializer)?;
                let paint_list = transmute::<_, Vec<ClippedShape>>(raw_paint_list);
                Ok(Self(paint_list.into()))
            }
        }
    }
    
    
    #[derive(Serialize, Deserialize)]
    #[repr(transparent)]
    struct SerializedShapeHelper(#[serde(with = "SerializedShape")] Shape);
    
    fn serialize_shape_vec<S: Serializer>(value: &Vec<Shape>, serializer: S) -> Result<S::Ok, S::Error> {
        unsafe {
            <Vec<SerializedShapeHelper> as Serialize>::serialize(transmute(value), serializer)
        }
    }
    
    fn deserialize_shape_vec<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Shape>, D::Error> {
        unsafe {
            Ok(transmute(<Vec<SerializedShapeHelper> as Deserialize>::deserialize(deserializer)?))
        }
    }
}