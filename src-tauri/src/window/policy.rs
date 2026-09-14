#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Desktop { Active, Inactive, Unknown }
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement { None, Hide, Bottom, Topmost }
pub fn decide(auto: bool, wanted: bool, visible: bool, topmost: bool, desktop: Desktop) -> Placement {
    if !wanted { return if visible { Placement::Hide } else { Placement::None }; }
    if !auto || desktop != Desktop::Active { return if topmost { Placement::Bottom } else { Placement::None }; }
    if !topmost || !visible { Placement::Topmost } else { Placement::None }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn hide_beats_desktop_recovery() { assert_eq!(decide(true,false,true,false,Desktop::Active),Placement::Hide); }
    #[test] fn active_promotes_once() { assert_eq!(decide(true,true,true,false,Desktop::Active),Placement::Topmost); assert_eq!(decide(true,true,true,true,Desktop::Active),Placement::None); }
    #[test] fn exiting_or_following_demotes() { assert_eq!(decide(true,true,true,true,Desktop::Inactive),Placement::Bottom); assert_eq!(decide(false,true,true,true,Desktop::Active),Placement::Bottom); }
    #[test] fn unknown_does_not_promote() { assert_eq!(decide(true,true,true,false,Desktop::Unknown),Placement::None); }
    #[test] fn hidden_remains_hidden() { assert_eq!(decide(true,false,false,true,Desktop::Active),Placement::None); }
}
