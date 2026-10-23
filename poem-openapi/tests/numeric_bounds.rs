use poem::{http::StatusCode, test::TestClient};
use poem_openapi::{
    Object, OpenApi, OpenApiService,
    param::Query,
    registry::{MetaSchema, Registry},
    types::{ParseFromJSON, Type},
    validation::{Maximum, Minimum, Validator},
};
use serde_json::json;

#[test]
fn integer_inputs_are_not_rounded_to_float_bounds() {
    assert!(!Maximum::new(9_007_199_254_740_992.0, false).check(&9_007_199_254_740_993_u64));
    assert!(Minimum::new(9_007_199_254_740_992.0, true).check(&9_007_199_254_740_993_u64));
    assert!(Maximum::new(18_446_744_073_709_551_616.0, true).check(&u64::MAX));
    assert!(Maximum::new(9_223_372_036_854_775_808.0, true).check(&i64::MAX));
}

#[derive(Object, Debug)]
struct LargeBounds {
    #[oai(validator(minimum(value = "9007199254740993")))]
    lower: u64,
    #[oai(validator(maximum(value = 18446744073709551615)))]
    upper: u64,
    #[oai(validator(minimum(value = -9223372036854775808)))]
    negative: i64,
}

#[test]
fn derive_preserves_integer_bounds_in_schema_and_validation() {
    let mut registry = Registry::new();
    LargeBounds::register(&mut registry);
    let schema = serde_json::to_value(&registry.schemas["LargeBounds"]).unwrap();
    assert_eq!(
        schema["properties"]["lower"]["minimum"],
        json!(9_007_199_254_740_993_u64)
    );
    assert_eq!(schema["properties"]["upper"]["maximum"], json!(u64::MAX));
    assert_eq!(schema["properties"]["negative"]["minimum"], json!(i64::MIN));
    assert!(
        LargeBounds::parse_from_json(Some(json!({
            "lower": 9_007_199_254_740_992_u64, "upper": u64::MAX, "negative": i64::MIN,
        })))
        .is_err()
    );
    let parsed = LargeBounds::parse_from_json(Some(json!({
        "lower": 9_007_199_254_740_993_u64, "upper": u64::MAX, "negative": i64::MIN,
    })))
    .unwrap();
    assert_eq!(
        (parsed.lower, parsed.upper, parsed.negative),
        (9_007_199_254_740_993, u64::MAX, i64::MIN)
    );
}

#[tokio::test]
async fn http_query_validation_preserves_integer_bounds() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn index(
            &self,
            #[oai(validator(minimum(value = 9007199254740993)))] value: Query<u64>,
        ) {
            let _ = value;
        }
    }

    let client = TestClient::new(OpenApiService::new(Api, "bounds", "1.0"));
    client
        .get("/?value=9007199254740992")
        .send()
        .await
        .assert_status(StatusCode::BAD_REQUEST);
    client
        .get("/?value=9007199254740993")
        .send()
        .await
        .assert_status_is_ok();
}

#[test]
fn public_schema_and_constructors_preserve_integer_precision() {
    use poem_openapi::validation::ValidatorMeta;

    let mut schema = MetaSchema::new("integer");
    Minimum::new(i64::MIN, true).update_meta(&mut schema);
    Maximum::new(u64::MAX, false).update_meta(&mut schema);
    let json = serde_json::to_value(&schema).unwrap();
    assert_eq!(json["minimum"].as_i64(), Some(i64::MIN));
    assert_eq!(json["maximum"].as_u64(), Some(u64::MAX));
    assert_eq!(json["exclusiveMinimum"], true);
    assert!(json.get("exclusiveMaximum").is_none());
    assert!(
        serde_json::to_string(&schema)
            .unwrap()
            .contains("18446744073709551615")
    );

    assert!(Maximum::new(u64::MAX, false).check(&u64::MAX));
    assert!(!Maximum::new(u64::MAX, true).check(&u64::MAX));
    assert!(Minimum::new(i64::MIN, false).check(&i64::MIN));
    assert!(!Minimum::new(i64::MIN, true).check(&i64::MIN));
    assert!(Maximum::new(9_007_199_254_740_993_u64, true).check(&9_007_199_254_740_992_u64));
    assert!(!Minimum::new(9_007_199_254_740_993_u64, false).check(&9_007_199_254_740_992_u64));
}

#[test]
fn mixed_numeric_types_use_exact_ordering() {
    use std::cmp::Ordering::{Equal, Greater, Less};

    use poem_openapi::registry::MetaSchemaNumber as Number;

    for (left, right, expected) in [
        (Number::from(-1_i64), Number::from(0_u64), Less),
        (Number::from(i64::MAX), Number::from(u64::MAX), Less),
        (Number::from(0_i64), Number::from(0_u64), Equal),
        (Number::from(2_u64), Number::from(1.5), Greater),
        (Number::from(1_u64), Number::from(1.5), Less),
        (Number::from(-2_i64), Number::from(-1.5), Less),
        (Number::from(-1_i64), Number::from(-1.5), Greater),
        (Number::from(0_u64), Number::from(-0.5), Greater),
        (Number::from(0_i64), Number::from(-0.0), Equal),
        (Number::from(0_i64), Number::from(f64::from_bits(1)), Less),
        (
            Number::from(0_i64),
            Number::from(-f64::from_bits(1)),
            Greater,
        ),
        (
            Number::from(i64::MIN),
            Number::from(-9_223_372_036_854_775_808.0),
            Equal,
        ),
        (
            Number::from(i64::MIN),
            Number::from((-9_223_372_036_854_775_808.0_f64).next_down()),
            Greater,
        ),
        (
            Number::from(i64::MAX),
            Number::from(9_223_372_036_854_775_808.0),
            Less,
        ),
        (
            Number::from(i64::MAX),
            Number::from(9_223_372_036_854_775_808.0_f64.next_down()),
            Greater,
        ),
        (
            Number::from(u64::MAX),
            Number::from(18_446_744_073_709_551_616.0),
            Less,
        ),
        (
            Number::from(u64::MAX),
            Number::from(18_446_744_073_709_551_616.0_f64.next_down()),
            Greater,
        ),
        (
            Number::from(9_007_199_254_740_993_u64),
            Number::from(9_007_199_254_740_992.0),
            Greater,
        ),
        (
            Number::from(-9_007_199_254_740_993_i64),
            Number::from(-9_007_199_254_740_992.0),
            Less,
        ),
        (Number::from(1.5_f32), Number::from(1.5_f64), Equal),
        (Number::from(1.25), Number::from(1.5), Less),
    ] {
        assert_eq!(
            left.partial_cmp(&right),
            Some(expected),
            "{left:?}, {right:?}"
        );
        assert_eq!(right.partial_cmp(&left), Some(expected.reverse()));
        assert_eq!(Maximum::new(right, false).check(&left), expected != Greater);
        assert_eq!(Maximum::new(right, true).check(&left), expected == Less);
        assert_eq!(Minimum::new(right, false).check(&left), expected != Less);
        assert_eq!(Minimum::new(right, true).check(&left), expected == Greater);
    }
}

#[test]
fn non_finite_bounds_and_values_are_rejected() {
    use poem_openapi::registry::MetaSchemaNumber;

    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for exclusive in [false, true] {
            assert!(!Maximum::new(value, exclusive).check(&0));
            assert!(!Minimum::new(value, exclusive).check(&0));
            assert!(!Maximum::new(0, exclusive).check(&value));
            assert!(!Minimum::new(0, exclusive).check(&value));
        }
        let schema = MetaSchema {
            minimum: Some(MetaSchemaNumber::from(value)),
            ..MetaSchema::new("number")
        };
        assert!(serde_json::to_value(&schema).is_err());
        assert!(serde_json::to_string(&schema).is_err());
    }
}

#[test]
fn standard_numeric_conversions_and_nonzero_values() {
    use std::num::NonZero;

    let upper = Maximum::new(100, false);
    assert!(upper.check(&100_i8));
    assert!(upper.check(&100_i16));
    assert!(upper.check(&100_i32));
    assert!(upper.check(&100_i64));
    assert!(upper.check(&100_isize));
    assert!(upper.check(&100_u8));
    assert!(upper.check(&100_u16));
    assert!(upper.check(&100_u32));
    assert!(upper.check(&100_u64));
    assert!(upper.check(&100_usize));
    assert!(upper.check(&100_f32));
    assert!(upper.check(&100_f64));
    assert!(upper.check(&NonZero::new(100_u64).unwrap()));
    assert!(Maximum::new(NonZero::new(u64::MAX).unwrap(), false).check(&u64::MAX));
}

#[test]
fn derive_fractional_exponent_and_exclusive_bounds() {
    #[derive(Object, Debug)]
    struct Fractional {
        #[oai(validator(minimum(value = -1.5), maximum(value = "1.5")))]
        value: i64,
        #[oai(validator(maximum(value = "1e3", exclusive)))]
        exponent: f64,
        #[oai(validator(maximum(value = 9007199254740993, exclusive)))]
        exact: u64,
    }

    let parsed = Fractional::parse_from_json(Some(json!({
        "value": -1, "exponent": 999.5, "exact": 9_007_199_254_740_992_u64,
    })))
    .unwrap();
    assert_eq!(
        (parsed.value, parsed.exponent, parsed.exact),
        (-1, 999.5, 9_007_199_254_740_992)
    );
    for value in [-2, 2] {
        assert!(
            Fractional::parse_from_json(Some(json!({
                "value": value, "exponent": 999.5, "exact": 9_007_199_254_740_992_u64,
            })))
            .is_err()
        );
    }
    assert!(
        Fractional::parse_from_json(Some(json!({
            "value": 0, "exponent": 1000.0, "exact": 9_007_199_254_740_992_u64,
        })))
        .is_err()
    );
    assert!(
        Fractional::parse_from_json(Some(json!({
            "value": 0, "exponent": 999.5, "exact": 9_007_199_254_740_993_u64,
        })))
        .is_err()
    );
}
