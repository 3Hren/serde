use std::default;
use serde_json;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Default {
    a1: i32,
    #[serde(default)]
    a2: i32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Rename {
    a1: i32,
    #[serde(rename="a3")]
    a2: i32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct FormatRename {
    a1: i32,
    #[serde(rename(xml= "a4", json="a5"))]
    a2: i32,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
enum SerEnum<A> {
    Map {
        a: i8,
        #[serde(rename(xml= "c", json="d"))]
        b: A,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct SkipSerializingFields<A: default::Default> {
    a: i8,
    #[serde(skip_serializing, default)]
    b: A,
}

#[test]
fn test_default() {
    let deserialized_value: Default = serde_json::from_str(&"{\"a1\":1,\"a2\":2}").unwrap();
    assert_eq!(deserialized_value, Default { a1: 1, a2: 2 });

    let deserialized_value: Default = serde_json::from_str(&"{\"a1\":1}").unwrap();
    assert_eq!(deserialized_value, Default { a1: 1, a2: 0 });
}

#[test]
fn test_rename() {
    let value = Rename { a1: 1, a2: 2 };
    let serialized_value = serde_json::to_string(&value).unwrap();
    assert_eq!(serialized_value, "{\"a1\":1,\"a3\":2}");

    let deserialized_value: Rename = serde_json::from_str(&serialized_value).unwrap();
    assert_eq!(value, deserialized_value);
}

#[test]
fn test_format_rename() {
    let value = FormatRename { a1: 1, a2: 2 };
    let serialized_value = serde_json::to_string(&value).unwrap();
    assert_eq!(serialized_value, "{\"a1\":1,\"a5\":2}");

    let deserialized_value = serde_json::from_str("{\"a1\":1,\"a5\":2}").unwrap();
    assert_eq!(value, deserialized_value);
}

#[test]
fn test_enum_format_rename() {
    let s1 = String::new();
    let value = SerEnum::Map { a: 0i8, b: s1 };
    let serialized_value = serde_json::to_string(&value).unwrap();
    let ans = "{\"Map\":{\"a\":0,\"d\":\"\"}}";
    assert_eq!(serialized_value, ans);

    let deserialized_value = serde_json::from_str(ans).unwrap();
    assert_eq!(value, deserialized_value);
}

#[test]
fn test_skip_serializing_fields() {
    let value = SkipSerializingFields { a: 1, b: 2 };
    let serialized_value = serde_json::to_string(&value).unwrap();
    assert_eq!(serialized_value, "{\"a\":1}");

    let deserialized_value: SkipSerializingFields<_> = serde_json::from_str(&serialized_value).unwrap();
    assert_eq!(SkipSerializingFields { a: 1, b: 0 }, deserialized_value);
}
