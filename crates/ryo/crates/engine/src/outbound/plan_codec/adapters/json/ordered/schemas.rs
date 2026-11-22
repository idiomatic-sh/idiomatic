//! A JSON object read and written with its keys in order: a plan's maps (an `extend`'s values, an ordered step's
//! output, the rules' emits) are in the order the query or the pack wrote them.

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::marker::PhantomData;

/// A JSON object as its entries, in order.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::outbound::plan_codec::adapters::json) struct OrderedMap<V>(pub Vec<(String, V)>);

impl<V: Serialize> Serialize for OrderedMap<V> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

/// Reads an object's entries in order.
struct OrderedVisitor<V>(PhantomData<V>);

impl<'de, V: Deserialize<'de>> Visitor<'de> for OrderedVisitor<V> {
    type Value = OrderedMap<V>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut entries = Vec::with_capacity(access.size_hint().unwrap_or_default());
        while let Some(entry) = access.next_entry()? {
            entries.push(entry);
        }
        Ok(OrderedMap(entries))
    }
}

impl<'de, V: Deserialize<'de>> Deserialize<'de> for OrderedMap<V> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(OrderedVisitor(PhantomData))
    }
}
