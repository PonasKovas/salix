macro_rules! gen_from_impls_for_variants {
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
        )*
    };
}

pub(crate) use gen_from_impls_for_variants;
