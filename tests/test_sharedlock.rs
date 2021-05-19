use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::{thread, time};

use sharedlock_rs::SharedLock;

#[test]
fn test_dead_lock() {
    let lock = SharedLock::new(0);
    let guard1 = lock.write();
    assert!(guard1.is_ok());

    let guard2 = lock.write();
    assert!(guard2.is_err());
}

#[test]
fn test_lock_busy() {
    let lock = SharedLock::new(0);
    let wg = lock.write();
    assert!(wg.is_ok());

    let rg = lock.read();
    assert!(rg.is_err());
}

#[test]
fn test_common() {
    let lock = SharedLock::new(5);

    {
        let r1 = lock.read().unwrap();
        let r2 = lock.read().unwrap();
        assert_eq!(*r1, 5);
        assert_eq!(*r2, 5);
    }

    {
        let mut w = lock.write().unwrap();
        *w += 1;
        assert_eq!(*w, 6);
    }
}

#[test]
fn test_validity() {
    const NUM_READERS: u32 = 8;
    const NUM_WRITERS: u32 = 2;
    const SLEEP_TIME_SEC: time::Duration = time::Duration::from_secs(6);

    let lock = Arc::new(SharedLock::new(0));
    let count = Arc::new(AtomicU64::new(0));

    let shutdown = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::new();

    for _ in 0..NUM_WRITERS {
        let lock_clone = lock.clone();
        let count_clone = count.clone();
        let shutdown_clone = shutdown.clone();

        handles.push(thread::spawn(move || {
            while !shutdown_clone.load(Ordering::Acquire) {
                let mut guard = lock_clone.write().unwrap();
                *guard += 1;
                count_clone.fetch_add(1, Ordering::SeqCst);
                drop(guard);
            }
        }));
    }

    for _ in 0..NUM_READERS {
        let lock_clone = lock.clone();
        let count_clone = count.clone();
        let shutdown_clone = shutdown.clone();

        handles.push(thread::spawn(move || {
            while !shutdown_clone.load(Ordering::Acquire) {
                let guard = lock_clone.read().unwrap();
                assert_eq!(*guard, count_clone.load(Ordering::SeqCst));
            }
        }));
    }

    thread::sleep(SLEEP_TIME_SEC);

    shutdown.store(true, Ordering::Release);

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(count.load(Ordering::Acquire), *lock.write().unwrap());
}
