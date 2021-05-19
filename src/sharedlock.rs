use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{Error, Result};

// The writer lock bit.
const SHARED_LOCK_WRITER_BIT: u64 = 1u64 << 63;

unsafe impl<T> Send for SharedLock<T> {}
unsafe impl<T> Sync for SharedLock<T> {}

/*
 * A reader-writer lock
 */
pub struct SharedLock<T: ?Sized> {
    inner: AtomicU64,
    owner: AtomicU64,
    data: UnsafeCell<T>,
}

impl<T> SharedLock<T> {
    pub fn new(t: T) -> Self {
        SharedLock {
            inner: AtomicU64::default(),
            owner: AtomicU64::default(),
            data: UnsafeCell::new(t),
        }
    }
}

impl<T: ?Sized> SharedLock<T> {
    pub fn read(&self) -> Result<SharedLockReadGuard<'_, T>> {
        SharedLockReadGuard::new(self)
    }

    pub fn write(&self) -> Result<SharedLockWriteGuard<'_, T>> {
        SharedLockWriteGuard::new(self)
    }

    fn is_hold(&self) -> bool {
        let tid = self.owner.load(Ordering::Acquire);
        tid > 0 && tid == unsafe { libc::pthread_self() } as u64
    }

    fn set_owner_id(&self, tid: u64) {
        self.owner.store(tid, Ordering::Release);
    }
}

/*
 * RAII structure used to release the shared read access of a lock when dropped.
 * This structure is created by the read methods on SharedLock.
 */
pub struct SharedLockReadGuard<'a, T: ?Sized + 'a> {
    lock: &'a SharedLock<T>,
}

impl<'a, T: ?Sized> SharedLockReadGuard<'a, T> {
    fn new(lock: &'a SharedLock<T>) -> Result<SharedLockReadGuard<'a, T>> {
        if lock.is_hold() {
            return Err(Error::DeadLockError);
        }

        loop {
            let value = lock.inner.load(Ordering::Acquire);
            if value >= SHARED_LOCK_WRITER_BIT {
                continue;
            }

            if lock
                .inner
                .compare_exchange(value, value + 1, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }

        Ok(SharedLockReadGuard { lock })
    }
}

impl<T: ?Sized> Deref for SharedLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for SharedLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.inner.fetch_sub(1, Ordering::Release);
    }
}

/*
 * RAII structure used to release the exclusive write access of a lock when dropped.
 * This structure is created by the write methods on SharedLock.
 */
pub struct SharedLockWriteGuard<'a, T: ?Sized + 'a> {
    lock: &'a SharedLock<T>,
}

impl<'a, T: ?Sized> SharedLockWriteGuard<'a, T> {
    fn new(lock: &'a SharedLock<T>) -> Result<SharedLockWriteGuard<'a, T>> {
        if lock.is_hold() {
            return Err(Error::DeadLockError);
        }

        loop {
            let value = lock.inner.load(Ordering::Acquire);
            if value >= SHARED_LOCK_WRITER_BIT {
                continue;
            }

            if lock
                .inner
                .compare_exchange(
                    value,
                    value | SHARED_LOCK_WRITER_BIT,
                    Ordering::Release,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                break;
            }
        }

        if lock.owner.load(Ordering::Acquire) != 0 {
            return Err(Error::Poisoned);
        }
        lock.set_owner_id(unsafe { libc::pthread_self() } as u64);

        // wait for active readers.
        while lock.inner.load(Ordering::Acquire) != SHARED_LOCK_WRITER_BIT {}

        Ok(SharedLockWriteGuard { lock })
    }
}

impl<T: ?Sized> Deref for SharedLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for SharedLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        let value = self.lock.inner.load(Ordering::Acquire);

        if value != SHARED_LOCK_WRITER_BIT {
            panic!("write unlock inner value: {}", value);
        }

        // reset owner id.
        if !self.lock.is_hold() {
            panic!(
                "Poisoned!!! owner id: {}",
                self.lock.owner.load(Ordering::Acquire)
            );
        }
        self.lock.set_owner_id(0);

        self.lock
            .inner
            .fetch_sub(SHARED_LOCK_WRITER_BIT, Ordering::Release);
    }
}

impl<T: ?Sized> DerefMut for SharedLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}
