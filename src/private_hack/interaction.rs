use crate::private_hack::*;

#[derive(Clone, Default)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct InteractionSnapshot {
    pub clicked: Option<Id>,
    pub long_touched: Option<Id>,
    pub drag_started: Option<Id>,
    pub dragged: Option<Id>,
    pub drag_stopped: Option<Id>,
    pub hovered: IdSet,
    pub contains_pointer: IdSet,
}