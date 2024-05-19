/// Implement `sqlx::Type`, `sqlx::Encode` and `sqlx::Decode` for values that can be safely
/// transmuted to another `sqlx::Type`, but can't use `#[derive(sqlx::Type)]` i.e. Nested enums.
macro_rules! impl_sqlx_type {
    (<$db:ty> $in_ty:ty as $out_ty:ty) => {
        impl sqlx::Type<$db> for $in_ty {
            fn type_info() -> <$db as sqlx::Database>::TypeInfo {
                <$out_ty as sqlx::Type<$db>>::type_info()
            }

            fn compatible(ty: &<$db as sqlx::Database>::TypeInfo) -> bool {
                <$out_ty as sqlx::Type<$db>>::compatible(ty)
            }
        }

        impl sqlx::Encode<'_, $db> for $in_ty {
            fn encode_by_ref(&self, buf: &mut <$db as HasArguments<'_>>::ArgumentBuffer) -> IsNull {
                #[allow(clippy::transmute_ptr_to_ptr)]
                let out: &$out_ty = unsafe { std::mem::transmute(self) };
                <$out_ty as sqlx::Encode<$db>>::encode_by_ref(out, buf)
            }
        }

        impl sqlx::Decode<'_, $db> for $in_ty {
            fn decode(value: <$db as HasValueRef<'_>>::ValueRef) -> Result<Self, BoxDynError> {
                <$out_ty as sqlx::Decode<$db>>::decode(value)
                    .map(|v| unsafe { std::mem::transmute(v) })
            }
        }
    };
}

macro_rules! impl_display_debug {
    ($t:ty) => {
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                <$t as std::fmt::Debug>::fmt(self, f)
            }
        }
    };
}

pub(super) use impl_display_debug;
pub(super) use impl_sqlx_type;
