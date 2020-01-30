use std::io;

// Ported from https://github.com/mimblewimble/grin/blob/5e1fe44bedee3a48e3d0573a8ce5e4fd8f4e97c1/core/src/ser.rs

// serializer for io::Errorkind, originally auto-generated by serde-derive
// slightly modified to handle the #[non_exhaustive] tag on io::ErrorKind
pub fn serialize<S>(kind: io::ErrorKind, serializer: S) -> serde::export::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match kind {
        io::ErrorKind::NotFound => {
            serde::Serializer::serialize_unit_variant(serializer, "ErrorKind", 0u32, "NotFound")
        }
        io::ErrorKind::PermissionDenied => serde::Serializer::serialize_unit_variant(
            serializer,
            "ErrorKind",
            1u32,
            "PermissionDenied",
        ),
        io::ErrorKind::ConnectionRefused => serde::Serializer::serialize_unit_variant(
            serializer,
            "ErrorKind",
            2u32,
            "ConnectionRefused",
        ),
        io::ErrorKind::ConnectionReset => serde::Serializer::serialize_unit_variant(
            serializer,
            "ErrorKind",
            3u32,
            "ConnectionReset",
        ),
        io::ErrorKind::ConnectionAborted => serde::Serializer::serialize_unit_variant(
            serializer,
            "ErrorKind",
            4u32,
            "ConnectionAborted",
        ),
        io::ErrorKind::NotConnected => {
            serde::Serializer::serialize_unit_variant(serializer, "ErrorKind", 5u32, "NotConnected")
        }
        io::ErrorKind::AddrInUse => {
            serde::Serializer::serialize_unit_variant(serializer, "ErrorKind", 6u32, "AddrInUse")
        }
        io::ErrorKind::AddrNotAvailable => serde::Serializer::serialize_unit_variant(
            serializer,
            "ErrorKind",
            7u32,
            "AddrNotAvailable",
        ),
        io::ErrorKind::BrokenPipe => {
            serde::Serializer::serialize_unit_variant(serializer, "ErrorKind", 8u32, "BrokenPipe")
        }
        io::ErrorKind::AlreadyExists => serde::Serializer::serialize_unit_variant(
            serializer,
            "ErrorKind",
            9u32,
            "AlreadyExists",
        ),
        io::ErrorKind::WouldBlock => {
            serde::Serializer::serialize_unit_variant(serializer, "ErrorKind", 10u32, "WouldBlock")
        }
        io::ErrorKind::InvalidInput => serde::Serializer::serialize_unit_variant(
            serializer,
            "ErrorKind",
            11u32,
            "InvalidInput",
        ),
        io::ErrorKind::InvalidData => {
            serde::Serializer::serialize_unit_variant(serializer, "ErrorKind", 12u32, "InvalidData")
        }
        io::ErrorKind::TimedOut => {
            serde::Serializer::serialize_unit_variant(serializer, "ErrorKind", 13u32, "TimedOut")
        }
        io::ErrorKind::WriteZero => {
            serde::Serializer::serialize_unit_variant(serializer, "ErrorKind", 14u32, "WriteZero")
        }
        io::ErrorKind::Interrupted => {
            serde::Serializer::serialize_unit_variant(serializer, "ErrorKind", 15u32, "Interrupted")
        }
        io::ErrorKind::Other => {
            serde::Serializer::serialize_unit_variant(serializer, "ErrorKind", 16u32, "Other")
        }
        io::ErrorKind::UnexpectedEof => serde::Serializer::serialize_unit_variant(
            serializer,
            "ErrorKind",
            17u32,
            "UnexpectedEof",
        ),
        // #[non_exhaustive] is used on the definition of ErrorKind for future compatability
        // That means match statements always need to match on _.
        // The downside here is that rustc won't be able to warn us if io::ErrorKind another
        // field is added to io::ErrorKind
        _ => serde::Serializer::serialize_unit_variant(serializer, "ErrorKind", 16u32, "Other"),
    }
}

// deserializer for io::Errorkind, originally auto-generated by serde-derive
pub fn deserialize<'de, D>(deserializer: D) -> serde::export::Result<io::ErrorKind, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[allow(non_camel_case_types)]
    enum Field {
        field0,
        field1,
        field2,
        field3,
        field4,
        field5,
        field6,
        field7,
        field8,
        field9,
        field10,
        field11,
        field12,
        field13,
        field14,
        field15,
        field16,
        field17,
    }
    struct FieldVisitor;
    impl<'de> serde::de::Visitor<'de> for FieldVisitor {
        type Value = Field;
        fn expecting(
            &self,
            formatter: &mut serde::export::Formatter,
        ) -> serde::export::fmt::Result {
            serde::export::Formatter::write_str(formatter, "variant identifier")
        }
        fn visit_u64<E>(self, value: u64) -> serde::export::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match value {
                0u64 => serde::export::Ok(Field::field0),
                1u64 => serde::export::Ok(Field::field1),
                2u64 => serde::export::Ok(Field::field2),
                3u64 => serde::export::Ok(Field::field3),
                4u64 => serde::export::Ok(Field::field4),
                5u64 => serde::export::Ok(Field::field5),
                6u64 => serde::export::Ok(Field::field6),
                7u64 => serde::export::Ok(Field::field7),
                8u64 => serde::export::Ok(Field::field8),
                9u64 => serde::export::Ok(Field::field9),
                10u64 => serde::export::Ok(Field::field10),
                11u64 => serde::export::Ok(Field::field11),
                12u64 => serde::export::Ok(Field::field12),
                13u64 => serde::export::Ok(Field::field13),
                14u64 => serde::export::Ok(Field::field14),
                15u64 => serde::export::Ok(Field::field15),
                16u64 => serde::export::Ok(Field::field16),
                17u64 => serde::export::Ok(Field::field17),
                _ => serde::export::Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Unsigned(value),
                    &"variant index 0 <= i < 18",
                )),
            }
        }
        fn visit_str<E>(self, value: &str) -> serde::export::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match value {
                "NotFound" => serde::export::Ok(Field::field0),
                "PermissionDenied" => serde::export::Ok(Field::field1),
                "ConnectionRefused" => serde::export::Ok(Field::field2),
                "ConnectionReset" => serde::export::Ok(Field::field3),
                "ConnectionAborted" => serde::export::Ok(Field::field4),
                "NotConnected" => serde::export::Ok(Field::field5),
                "AddrInUse" => serde::export::Ok(Field::field6),
                "AddrNotAvailable" => serde::export::Ok(Field::field7),
                "BrokenPipe" => serde::export::Ok(Field::field8),
                "AlreadyExists" => serde::export::Ok(Field::field9),
                "WouldBlock" => serde::export::Ok(Field::field10),
                "InvalidInput" => serde::export::Ok(Field::field11),
                "InvalidData" => serde::export::Ok(Field::field12),
                "TimedOut" => serde::export::Ok(Field::field13),
                "WriteZero" => serde::export::Ok(Field::field14),
                "Interrupted" => serde::export::Ok(Field::field15),
                "Other" => serde::export::Ok(Field::field16),
                "UnexpectedEof" => serde::export::Ok(Field::field17),
                _ => serde::export::Err(serde::de::Error::unknown_variant(value, VARIANTS)),
            }
        }
        fn visit_bytes<E>(self, value: &[u8]) -> serde::export::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match value {
                b"NotFound" => serde::export::Ok(Field::field0),
                b"PermissionDenied" => serde::export::Ok(Field::field1),
                b"ConnectionRefused" => serde::export::Ok(Field::field2),
                b"ConnectionReset" => serde::export::Ok(Field::field3),
                b"ConnectionAborted" => serde::export::Ok(Field::field4),
                b"NotConnected" => serde::export::Ok(Field::field5),
                b"AddrInUse" => serde::export::Ok(Field::field6),
                b"AddrNotAvailable" => serde::export::Ok(Field::field7),
                b"BrokenPipe" => serde::export::Ok(Field::field8),
                b"AlreadyExists" => serde::export::Ok(Field::field9),
                b"WouldBlock" => serde::export::Ok(Field::field10),
                b"InvalidInput" => serde::export::Ok(Field::field11),
                b"InvalidData" => serde::export::Ok(Field::field12),
                b"TimedOut" => serde::export::Ok(Field::field13),
                b"WriteZero" => serde::export::Ok(Field::field14),
                b"Interrupted" => serde::export::Ok(Field::field15),
                b"Other" => serde::export::Ok(Field::field16),
                b"UnexpectedEof" => serde::export::Ok(Field::field17),
                _ => {
                    let value = &serde::export::from_utf8_lossy(value);
                    serde::export::Err(serde::de::Error::unknown_variant(value, VARIANTS))
                }
            }
        }
    }
    impl<'de> serde::Deserialize<'de> for Field {
        #[inline]
        fn deserialize<D>(deserializer: D) -> serde::export::Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            serde::Deserializer::deserialize_identifier(deserializer, FieldVisitor)
        }
    }
    struct Visitor<'de> {
        marker: serde::export::PhantomData<io::ErrorKind>,
        lifetime: serde::export::PhantomData<&'de ()>,
    }
    impl<'de> serde::de::Visitor<'de> for Visitor<'de> {
        type Value = io::ErrorKind;
        fn expecting(
            &self,
            formatter: &mut serde::export::Formatter,
        ) -> serde::export::fmt::Result {
            serde::export::Formatter::write_str(formatter, "enum io::ErrorKind")
        }
        fn visit_enum<A>(self, data: A) -> serde::export::Result<Self::Value, A::Error>
        where
            A: serde::de::EnumAccess<'de>,
        {
            match match serde::de::EnumAccess::variant(data) {
                serde::export::Ok(val) => val,
                serde::export::Err(err) => {
                    return serde::export::Err(err);
                }
            } {
                (Field::field0, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::NotFound)
                }
                (Field::field1, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::PermissionDenied)
                }
                (Field::field2, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::ConnectionRefused)
                }
                (Field::field3, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::ConnectionReset)
                }
                (Field::field4, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::ConnectionAborted)
                }
                (Field::field5, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::NotConnected)
                }
                (Field::field6, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::AddrInUse)
                }
                (Field::field7, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::AddrNotAvailable)
                }
                (Field::field8, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::BrokenPipe)
                }
                (Field::field9, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::AlreadyExists)
                }
                (Field::field10, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::WouldBlock)
                }
                (Field::field11, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::InvalidInput)
                }
                (Field::field12, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::InvalidData)
                }
                (Field::field13, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::TimedOut)
                }
                (Field::field14, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::WriteZero)
                }
                (Field::field15, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::Interrupted)
                }
                (Field::field16, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::Other)
                }
                (Field::field17, variant) => {
                    match serde::de::VariantAccess::unit_variant(variant) {
                        serde::export::Ok(val) => val,
                        serde::export::Err(err) => {
                            return serde::export::Err(err);
                        }
                    };
                    serde::export::Ok(io::ErrorKind::UnexpectedEof)
                }
            }
        }
    }
    const VARIANTS: &[&str] = &[
        "NotFound",
        "PermissionDenied",
        "ConnectionRefused",
        "ConnectionReset",
        "ConnectionAborted",
        "NotConnected",
        "AddrInUse",
        "AddrNotAvailable",
        "BrokenPipe",
        "AlreadyExists",
        "WouldBlock",
        "InvalidInput",
        "InvalidData",
        "TimedOut",
        "WriteZero",
        "Interrupted",
        "Other",
        "UnexpectedEof",
    ];
    serde::Deserializer::deserialize_enum(
        deserializer,
        "ErrorKind",
        VARIANTS,
        Visitor {
            marker: serde::export::PhantomData::<io::ErrorKind>,
            lifetime: serde::export::PhantomData,
        },
    )
}
