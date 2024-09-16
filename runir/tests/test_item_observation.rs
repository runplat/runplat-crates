use std::time::Duration;
use runir::store::Item;

#[test]
fn test_item_borrow_resource_multi_thread() {
    let mut item = Item::from(String::from("HELLO WORLD"));
    let observe = item.observe();
    let mut cloned = item.clone();
    let _ = std::thread::Builder::new().spawn(move || {
        if let Some(item) = cloned.borrow_mut::<String>() {
            item.extend(['t', 'e', 's', 't']);
        }
    });

    std::thread::sleep(Duration::from_millis(100));
    assert!(observe.wait());
    let item = item.borrow::<String>().expect("should exist");
    assert_eq!("HELLO WORLDtest", item);
}

#[test]
fn test_item_borrow_resource_multi_thread_observe_timeout() {
    let mut item = Item::from(String::from("HELLO WORLD"));
    let observe = item.observe_with_timeout(Duration::from_millis(100));
    let _ = std::thread::Builder::new().spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
    });
    assert!(!observe.wait());
    let item = item.borrow::<String>().expect("should exist");
    assert_eq!("HELLO WORLD", item);
}
