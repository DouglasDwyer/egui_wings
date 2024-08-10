use crate::private_hack::*;

pub struct AnimationManager {
    bools: IdMap<BoolAnim>,
    values: IdMap<ValueAnim>,
}

#[derive(Clone, Debug)]
struct BoolAnim {
    last_value: f32,
    last_tick: f64,
}

#[derive(Clone, Debug)]
struct ValueAnim {
    from_value: f32,

    to_value: f32,

    /// when did `value` last toggle?
    toggle_time: f64,
}