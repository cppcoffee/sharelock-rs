# sharedlock-rs

## Introduce

ShareLock is a spin read/write lock library implemented using rust.

The RAII guards returned from the locking methods implement Deref (and DerefMut for the write methods) to allow access to the content of the lock.


## Examples

```rust
use sharedlock_rs::SharedLock;

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
```


## Reference

[https://en.wikipedia.org/wiki/Readers%E2%80%93writer_lock](https://en.wikipedia.org/wiki/Readers%E2%80%93writer_lock)

