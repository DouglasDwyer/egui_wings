use crate::private_hack::*;

#[derive(Clone, Default)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct WidgetHits {
    pub contains_pointer: Vec<crate::private_hack::widget_rect::WidgetRect>,
    pub click: Option<crate::private_hack::widget_rect::WidgetRect>,
    pub drag: Option<crate::private_hack::widget_rect::WidgetRect>,
}

