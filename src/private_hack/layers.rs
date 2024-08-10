use crate::private_hack::*;

#[derive(Clone, Default)]
pub struct PaintList(Vec<ClippedShape>);

impl PaintList {
    pub fn as_inner(&self) -> &Vec<ClippedShape> {
        &self.0
    }

    pub fn as_inner_mut(&mut self) -> &mut Vec<ClippedShape> {
        &mut self.0
    }

    pub fn from_inner(list: Vec<ClippedShape>) -> Self {
        Self(list)
    }
}

pub type GraphicLayersInner = [IdMap<PaintList>; 6];

#[derive(Clone, Default)]
pub struct GraphicLayers(GraphicLayersInner);

impl GraphicLayers {
    pub fn as_inner(&self) -> &GraphicLayersInner {
        &self.0
    }

    pub fn as_inner_mut(&mut self) -> &mut GraphicLayersInner {
        &mut self.0
    }

    pub fn from_inner(maps: GraphicLayersInner) -> Self {
        Self(maps)
    }
}
