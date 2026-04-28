use crate::Position;
use crate::acs_text::base::*;
use crate::acs_text::{KeyValuePair, Kuid, Value};

#[test]
fn test_kuid_version_logic() {
    let mut kuid = Kuid {
        user_id: 123,
        content_id: 456,
        version: None,
        range: crate::Range::default(),
    };

    assert!(kuid.can_increment());
    assert!(!kuid.can_decrement());
    assert_eq!(kuid.to_string(), "<kuid:123:456>");

    kuid.increment();
    assert_eq!(kuid.version, Some(2));
    assert!(kuid.can_decrement());
    assert_eq!(kuid.to_string(), "<kuid2:123:456:2>");

    kuid.decrement();
    assert_eq!(kuid.version, Some(1));
    assert!(!kuid.can_decrement());
    assert_eq!(kuid.to_string(), "<kuid2:123:456:1>");

    kuid.increment();
    assert_eq!(kuid.version, Some(2));
}

#[test]
fn test_acs_text_find_kuids() {
    let kuid1 = Kuid {
        user_id: 1,
        content_id: 1,
        version: None,
        range: crate::Range {
            start: Position {
                line: 0,
                character: 10,
            },
            end: Position {
                line: 0,
                character: 20,
            },
        },
    };

    let kuid2 = Kuid {
        user_id: 1,
        content_id: 1,
        version: Some(2),
        range: crate::Range {
            start: Position {
                line: 1,
                character: 10,
            },
            end: Position {
                line: 1,
                character: 20,
            },
        },
    };

    let r1 = kuid1.range;
    let r2 = kuid2.range;

    let acs_text = AcsText {
        key_value_pairs: vec![
            KeyValuePair {
                key: "k1".to_string(),
                key_range: crate::Range::default(),
                value: Some(Value::Kuid(kuid1, r1)),
                range: crate::Range::default(),
            },
            KeyValuePair {
                key: "k2".to_string(),
                key_range: crate::Range::default(),
                value: Some(Value::Kuid(kuid2, r2)),
                range: crate::Range::default(),
            },
        ],
        range: crate::Range::default(),
        src: "".to_string(),
    };

    let kuids = acs_text.find_all_kuids();
    assert_eq!(kuids.len(), 2);

    // Find at cursor
    let found = acs_text.find_kuid_at(Position {
        line: 0,
        character: 15,
    });
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.user_id, 1);
    assert_eq!(found.content_id, 1);
    assert_eq!(found.version, None);

    let found_none = acs_text.find_kuid_at(Position {
        line: 0,
        character: 25,
    });
    assert!(found_none.is_none());
}

#[test]
fn test_get_text_at() {
    let acs_text = AcsText {
        key_value_pairs: vec![],
        range: crate::Range::default(),
        src: "hello world test".to_string(),
    };

    // Test position in 'hello'
    let pos = Position {
        line: 0,
        character: 1,
    };
    assert_eq!(acs_text.get_text_at(pos), Some("hello".to_string()));

    // Test position in 'world'
    let pos = Position {
        line: 0,
        character: 7,
    };
    assert_eq!(acs_text.get_text_at(pos), Some("world".to_string()));

    // Test position in whitespace (between 'hello' and 'world')
    let pos = Position {
        line: 0,
        character: 5,
    };
    assert_eq!(acs_text.get_text_at(pos), Some("hello".to_string()));

    // Test position in whitespace (between 'world' and 'test')
    let pos = Position {
        line: 0,
        character: 11,
    };
    assert_eq!(acs_text.get_text_at(pos), Some("world".to_string()));
}

#[test]
fn test_get_kvp_to_position() {
    let kvp1 = KeyValuePair {
        key: "k1".to_string(),
        key_range: crate::Range::default(),
        value: Some(Value::Container(
            vec![KeyValuePair {
                key: "k2".to_string(),
                key_range: crate::Range::default(),
                value: None,
                range: crate::Range {
                    start: Position {
                        line: 0,
                        character: 10,
                    },
                    end: Position {
                        line: 0,
                        character: 20,
                    },
                },
            }],
            crate::Range::default(),
            crate::Range::default(),
        )),
        range: crate::Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 30,
            },
        },
    };

    let acs_text = AcsText {
        key_value_pairs: vec![kvp1.clone()],
        range: crate::Range::default(),
        src: "".to_string(),
    };

    // Test position in kvp2
    let pos = Position {
        line: 0,
        character: 15,
    };
    let path = acs_text.get_kvp_to_position(pos);
    assert_eq!(path.len(), 2);
    assert_eq!(path[0].key, "k1");
    assert_eq!(path[1].key, "k2");
}
