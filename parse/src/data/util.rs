macro_rules! impl_display_debug {
    ($t:ty) => {
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                <$t as std::fmt::Debug>::fmt(self, f)
            }
        }
    };
}

/// Implement `ToSql` and `FromSql` for a type by transmuting it to another type.
/// This should only be considered safe for enums, where they use `#[repr($out_ty)]`.
macro_rules! diesel_tosql_transmute {
    ($in_ty:ty, $out_ty:ty, $sql_ty:ty) => {
        impl diesel::serialize::ToSql<$sql_ty, diesel::sqlite::Sqlite> for $in_ty {
            fn to_sql<'b>(
                &'b self,
                out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>,
            ) -> diesel::serialize::Result {
                out.set_value(unsafe { std::mem::transmute::<$in_ty, $out_ty>(*self) });
                Ok(diesel::serialize::IsNull::No)
            }
        }

        impl diesel::deserialize::FromSql<$sql_ty, diesel::sqlite::Sqlite> for $in_ty {
            fn from_sql(
                bytes: <diesel::sqlite::Sqlite as diesel::backend::Backend>::RawValue<'_>,
            ) -> diesel::deserialize::Result<Self> {
                let int: $out_ty = <$out_ty as diesel::deserialize::FromSql<
                    $sql_ty,
                    diesel::sqlite::Sqlite,
                >>::from_sql(bytes)?;
                Ok(unsafe { std::mem::transmute(int) })
            }
        }
    };
}

#[allow(unused)]
macro_rules! diesel_tosql_json {
    ($t:ty) => {
        impl diesel::deserialize::FromSql<diesel::sql_types::Text, diesel::sqlite::Sqlite> for $t {
            fn from_sql(
                bytes: <diesel::sqlite::Sqlite as diesel::backend::Backend>::RawValue<'_>,
            ) -> diesel::deserialize::Result<Self> {
                let t =
                    <String as FromSql<diesel::sql_types::Text, diesel::sqlite::Sqlite>>::from_sql(
                        bytes,
                    )?;
                Ok(serde_json::from_str(&t)?)
            }
        }

        impl diesel::serialize::ToSql<diesel::sql_types::Text, diesel::sqlite::Sqlite> for $t {
            fn to_sql<'b>(
                &'b self,
                out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>,
            ) -> diesel::serialize::Result {
                let s = serde_json::to_string(&self)?;
                out.set_value(s);
                Ok(diesel::serialize::IsNull::No)
            }
        }
    };
}

pub(crate) use diesel_tosql_transmute;
pub(crate) use impl_display_debug;
