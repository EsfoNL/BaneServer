#[macro_export]
macro_rules! StrEnum {
    ($name:ident, [$($mem:ident => $str:literal),*]) => {
        enum $name {
            $($mem),*
        }

        impl FromStr for $name {
            type Err = ();
            fn from_str(value: &str) -> Result<Self, ()> {
                Ok(match value {
                    $($str => $name::$mem,)*
                    _ => return Err(()),
                })
            }
        }

        impl $name {
            fn to_str(&self) -> &'static str {
                match self {
                    $($name::$mem => $str,)*
                }
            }
        }
    };
}
