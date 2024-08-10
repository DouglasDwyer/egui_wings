use crate::private_hack::*;

#[derive(Clone)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct WidgetRect {
    pub id: Id,
    pub layer_id: LayerId,
    pub rect: Rect,
    pub interact_rect: Rect,
    pub sense: Sense,
    pub enabled: bool,
}

#[derive(Clone, Default)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct WidgetRects {
    by_layer: ahash::HashMap<LayerId, Vec<WidgetRect>>,
    by_id: IdMap<(usize, WidgetRect)>,
    infos: IdMap<WidgetInfo>,
}