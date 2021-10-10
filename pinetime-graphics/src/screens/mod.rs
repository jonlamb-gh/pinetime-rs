pub mod watch_face;
pub use watch_face::{WatchFace, WatchFaceResources};

// some trait, ScreenExt: Drawable
// on_focus
// off_focus
//
// update(Self::Resources)
//
// (use eg::ContainsPoint trait)
// handle_event(Event) -> Forward/SomeAction (like switch to ScreenFoo)
//   touch/gesture
//   button press

// enum Screen { WatchFace(..), ... }
