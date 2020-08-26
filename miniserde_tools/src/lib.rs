pub extern crate miniserde;

#[macro_export]
macro_rules! miniserialize_for_display {
    ($t:ty) => {
        impl miniserde::Serialize for $t {
            fn begin(&self) -> $crate::miniserde::ser::Fragment {
                $crate::miniserde::ser::Fragment::Str(::std::borrow::Cow::Owned(format!(
                    "{}",
                    self
                )))
            }
        }
    };
}
#[macro_export]
macro_rules! minideserialize_for_fromstr {
    ($t:ty) => {
        impl miniserde::Deserialize for $t {
            fn begin(out: &mut Option<Self>) -> &mut dyn $crate::miniserde::de::Visitor {
                make_place!(Place);
                impl $crate::miniserde::de::Visitor for Place<$t> {
                    fn string(&mut self, s: &str) -> miniserde::Result<()> {
                        match ::std::str::FromStr::from_str(s) {
                            Ok(x) => {
                                self.out = Some(x);
                                Ok(())
                            }
                            Err(_) => Err(miniserde::Error),
                        }
                    }
                }
                Place::new(out)
            }
        }
    };
}
