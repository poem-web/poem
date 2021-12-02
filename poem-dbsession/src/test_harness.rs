#![allow(dead_code)]

use std::{collections::BTreeMap, time::Duration};

use poem::session::SessionStorage;

pub(crate) async fn test_storage(storage: impl SessionStorage) {
    let mut entries1 = BTreeMap::new();
    entries1.insert("a".to_string(), "1".to_string());
    entries1.insert("b".to_string(), "2".to_string());

    let mut entries2 = BTreeMap::new();
    entries2.insert("c".to_string(), "3".to_string());
    entries2.insert("d".to_string(), "4".to_string());

    let mut entries3 = BTreeMap::new();
    entries3.insert("e".to_string(), "5".to_string());
    entries3.insert("f".to_string(), "6".to_string());

    storage
        .update_session("a1", &entries1, Some(Duration::from_secs(3)))
        .await
        .unwrap();
    storage.update_session("a2", &entries2, None).await.unwrap();
    assert_eq!(
        storage.load_session("a1").await.unwrap().as_ref(),
        Some(&entries1)
    );
    assert_eq!(
        storage.load_session("a2").await.unwrap().as_ref(),
        Some(&entries2)
    );

    tokio::time::sleep(Duration::from_secs(5)).await;

    assert_eq!(storage.load_session("a1").await.unwrap().as_ref(), None);
    assert_eq!(
        storage.load_session("a2").await.unwrap().as_ref(),
        Some(&entries2)
    );

    storage.update_session("a2", &entries3, None).await.unwrap();
    assert_eq!(
        storage.load_session("a2").await.unwrap().as_ref(),
        Some(&entries3)
    );

    storage.remove_session("a2").await.unwrap();
    assert_eq!(storage.load_session("a2").await.unwrap().as_ref(), None);
}
