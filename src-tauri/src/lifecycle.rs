use std::time::{Instant,Duration};
// Visual notices can expire without changing the user's saved visibility preference.
pub fn advance(visible:bool,temporary:&mut Option<Instant>,leaving:&mut Option<Instant>,now:Instant)->(bool,bool,bool){
 let expired=temporary.is_some_and(|t|now>=t);
 if expired {*temporary=None;if !visible{*leaving=Some(now+Duration::from_millis(220));}}
 let hide=leaving.is_some_and(|t|now>=t);
 if hide{*leaving=None;}
 (expired,expired&&!visible,hide)
}
#[cfg(test)] mod tests{
 use super::*;
 #[test] fn hidden_returns_to_hidden(){let now=Instant::now();let mut t=Some(now);let mut l=None;assert_eq!(advance(false,&mut t,&mut l,now),(true,true,false));assert_eq!(advance(false,&mut t,&mut l,now+Duration::from_millis(221)),(false,false,true));}
 #[test] fn visible_stays_visible(){let now=Instant::now();let mut t=Some(now);let mut l=None;assert_eq!(advance(true,&mut t,&mut l,now),(true,false,false));assert!(l.is_none());}
 #[test] fn overlapping_notifications_extend_expiry(){let now=Instant::now();let mut t=Some(now+Duration::from_secs(8));let mut l=None;assert_eq!(advance(false,&mut t,&mut l,now),(false,false,false));assert!(t.is_some());}
 #[test] fn user_show_overrides_temporary_hide(){let now=Instant::now();let mut t=None;let mut l=None;assert_eq!(advance(true,&mut t,&mut l,now+Duration::from_secs(30)),(false,false,false));}
}
