use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::{self, Display},
    hash::{BuildHasherDefault, DefaultHasher},
    str::FromStr,
};

use poem_openapi::types::{MaybeUndefined, ToJSON};
use serde_json::json;

type TestHasher = BuildHasherDefault<DefaultHasher>;

#[test]
fn sequence_json_preserves_order_nulls_and_omits_undefined() {
    use MaybeUndefined::{Null, Undefined, Value};

    let values = [Value(3), Undefined, Null, Value(1), Value(3)];
    let expected = Some(json!([3, null, 1, 3]));
    assert_eq!(values.to_json(), expected);
    assert_eq!(values.as_slice().to_json(), expected);
    assert_eq!(values.to_vec().to_json(), expected);
    assert_eq!(
        vec![Some(3), None, Some(1)].to_json(),
        Some(json!([3, null, 1]))
    );

    let empty: [MaybeUndefined<i32>; 0] = [];
    assert_eq!(empty.to_json(), Some(json!([])));
    assert_eq!(empty.as_slice().to_json(), Some(json!([])));
    assert_eq!(empty.to_vec().to_json(), Some(json!([])));
    assert_eq!([Undefined::<i32>; 3].to_json(), Some(json!([])));
    assert_eq!(
        vec![vec![1, 2], vec![]].to_json(),
        Some(json!([[1, 2], []]))
    );
}

#[test]
fn set_json_preserves_iteration_order_and_omits_undefined() {
    use MaybeUndefined::{Null, Undefined, Value};

    let values = [Value(3), Undefined, Null, Value(1), Value(3)];
    let ordered = BTreeSet::from(values);
    assert_eq!(ordered.to_json(), Some(json!([null, 1, 3])));

    let hashed: HashSet<_, TestHasher> = values.into_iter().collect();
    let expected: Vec<_> = hashed.iter().filter_map(ToJSON::to_json).collect();
    assert_eq!(hashed.to_json(), Some(serde_json::Value::Array(expected)));
    assert_eq!(BTreeSet::<i32>::new().to_json(), Some(json!([])));
    assert_eq!(
        HashSet::<i32, TestHasher>::default().to_json(),
        Some(json!([]))
    );
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
struct CollidingKey(u8);

impl Display for CollidingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert_ne!(self.0, 3, "omitted values must not stringify their keys");
        f.write_str("same")
    }
}

impl FromStr for CollidingKey {
    type Err = std::num::ParseIntError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}

#[test]
fn map_json_preserves_nulls_and_omits_undefined() {
    use MaybeUndefined::{Null, Undefined, Value};

    let values = [(1, Value(3)), (2, Undefined), (3, Null)];
    let expected = Some(json!({"1": 3, "3": null}));
    assert_eq!(BTreeMap::from(values).to_json(), expected);
    let hashed: HashMap<_, _, TestHasher> = values.into_iter().collect();
    assert_eq!(hashed.to_json(), expected);
    assert_eq!(BTreeMap::<String, i32>::new().to_json(), Some(json!({})));
    assert_eq!(
        HashMap::<String, i32, TestHasher>::default().to_json(),
        Some(json!({}))
    );
}

#[test]
fn map_json_preserves_last_serialized_value_for_colliding_keys() {
    use MaybeUndefined::{Undefined, Value};

    let ordered = BTreeMap::from([
        (CollidingKey(1), Value(10)),
        (CollidingKey(2), Value(20)),
        (CollidingKey(3), Undefined),
    ]);
    assert_eq!(ordered.to_json(), Some(json!({"same": 20})));

    let hashed: HashMap<_, _, TestHasher> = ordered.into_iter().collect();
    let last_value = hashed.values().filter_map(ToJSON::to_json).last().unwrap();
    assert_eq!(hashed.to_json(), Some(json!({"same": last_value})));
}

#[test]
fn hash_collection_serialization_does_not_require_a_build_hasher() {
    let set = HashSet::<i32, ()>::with_hasher(());
    let map = HashMap::<String, i32, ()>::with_hasher(());
    assert_eq!(set.to_json(), Some(json!([])));
    assert_eq!(map.to_json(), Some(json!({})));
}
