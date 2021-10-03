use crate::embedded_graphics::draw_target::{DrawTarget, DrawTargetExt};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum RefreshDirection {
    Up,
    Down,
    /*
    Left,
    Right,
    LeftAnim,
    RightAnim,
    */
}

pub trait AnimatedDisplay: DrawTarget + DrawTargetExt {
    type Error;
    // associate const for refresh rate / animation ticks per draw thing

    // DrawTarget methods will update the effects
    // TODO - maybe set, clear, in-progress methods
    // currently the active aninmation runs to completion, any new set's are ignored
    fn set_refresh_direction(&mut self, refresh_dir: RefreshDirection);

    fn update_animations(&mut self) -> Result<(), <Self as AnimatedDisplay>::Error>;
}
