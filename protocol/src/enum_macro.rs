macro_rules! gen_from_impls_for_variants {
    (
        $(#[$attrs:meta])*
        $v:vis enum $enum_name:ident {
            $( $variant:ident($field:ty) ),*
            $(,)?
        }
    ) => {
        $(#[$attrs])*
        $v enum $enum_name {
            $( $variant($field) ),*
        }

        impl $enum_name {
        	fn variant_name(&self) -> &'static str {
		        match *self {
		        	$( Self::$variant(_) => stringify!($variant), )*
		        }
		    }
        }

        $(
            impl From<$field> for $enum_name {
                fn from(value: $field) -> Self {
                    $enum_name::$variant(value)
                }
            }

			impl WriteMessage for $field {
				fn write(
					&self,
					to: impl AsyncWrite + Send,
				) -> impl Future<Output = Result<usize, std::io::Error>> + Send {
					message::encode(self.into(), to)
				}
			}
        )*
    };
}

pub(crate) use gen_from_impls_for_variants;
