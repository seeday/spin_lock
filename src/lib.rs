#![feature(optin_builtin_traits)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::ptr;
use std::mem;

#[derive(Debug)]
struct InnerSpinner {
    spinner: AtomicBool,
}

impl InnerSpinner {
    fn new() -> InnerSpinner {
        InnerSpinner {
            spinner: AtomicBool::new(false),
        }
    }

    unsafe fn lock(&self) {
        while self.spinner
            .compare_and_swap(false, true, Ordering::Acquire)
        {}
    }

    unsafe fn try_lock(&self) -> bool {
        !self.spinner
            .compare_and_swap(false, true, Ordering::Acquire)
    }

    unsafe fn unlock(&self) {
        self.spinner.store(false, Ordering::Release);
    }
}

#[derive(Debug)]
pub struct Lock<T: ?Sized> {
    inner: Box<InnerSpinner>,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for Lock<T> {}
unsafe impl<T: ?Sized + Send> Sync for Lock<T> {}

pub struct LockGuard<'a, T: ?Sized + 'a> {
    __lock: &'a Lock<T>,
}

impl<'a, T: ?Sized> !Send for LockGuard<'a, T> {}
unsafe impl<'a, T: ?Sized + Sync> Sync for LockGuard<'a, T> {}

impl<T> Lock<T> {
    pub fn new(value: T) -> Lock<T> {
        Lock {
            inner: Box::new(InnerSpinner::new()),
            data: UnsafeCell::new(value),
        }
    }
}

impl<T: ?Sized> Lock<T> {
    pub fn lock(&self) -> LockGuard<T> {
        unsafe {
            self.inner.lock();
            LockGuard::new(self)
        }
    }

    pub fn into_inner(self) -> Result<T, ()>
    where
        T: Sized,
    {
        unsafe {
            let (inner, data) = {
                let Lock {
                    ref inner,
                    ref data,
                } = self;
                (ptr::read(inner), ptr::read(data))
            };
            mem::forget(self);
            drop(inner);
            Ok(data.into_inner())
        }
    }
}

impl<'spinl, T: ?Sized> LockGuard<'spinl, T> {
    #[inline]
    unsafe fn new(lock: &'spinl Lock<T>) -> LockGuard<'spinl, T> {
        LockGuard { __lock: lock }
    }

    pub fn unwrap(self) -> Self {
        self
    }
}

impl<'spinl, T: ?Sized> Deref for LockGuard<'spinl, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.__lock.data.get() }
    }
}

impl<'spinl, T: ?Sized> DerefMut for LockGuard<'spinl, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.__lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for LockGuard<'a, T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.__lock.inner.unlock();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let l = Lock::new(0u32);
        {
            let mut v = l.lock();
            *v += 1;
        }

        assert_eq!(1, *l.lock());
    }

    #[test]
    fn inner() {
        let l = Lock::new(0u32);
        {
            let mut v = l.lock();
            *v += 1;
        }

        assert_eq!(1, l.into_inner());
    }
}
