macro_rules! from_variants {
    (
        $(#[$attrs:meta])*
        $v:vis enum $enum_name:ident {
            $( $variant:ident($struct:ty) ),*
            $(,)?
        }
    ) => {
        $(#[$attrs])*
        $v enum $enum_name {
            $( $variant($struct) ),*
        }

        $(
            impl From<$struct> for $enum_name {
                fn from(value: $struct) -> Self {
                    $enum_name::$variant(value)
                }
            }
            impl IntoMessage<$enum_name> for $struct {
                fn into_message(self) -> impl Message<$enum_name> {
                    $enum_name::from(self)
                }
            }
        )*
    };
}

macro_rules! message {
    ( $enum_name:path => $direction:path ) => {
        impl crate::message::Message<$direction> for $enum_name {
            fn write(&self) -> Vec<u8> {
                crate::message::encode(self)
            }
            fn read(from: &[u8]) -> Result<Self, crate::Error> {
                crate::message::decode(from)
            }
        }
    };
}

pub(crate) use {from_variants, message};
