//! Thread specific operations.

use crate::error::Result;
use crate::raw::{read, read_mib};

option! {
    allocatedp allocatedp_mib[ name: b"thread.allocatedp\0", mib: 2 ] => *mut u64 |
    ops:  |
    test: allocatedp_read_test |
    docs:
    /// Access to the total number of bytes allocated by the current thread.
    ///
    /// This cumulative counter includes allocations that have since been freed.
    ///
    /// Each call to [`Self::read`] returns a pointer to the calling thread's
    /// counter. [`ThreadLocal::get`] reads it without another `mallctl` call.
    ///
    /// # Example
    ///
    /// ```
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// use tikv_jemalloc_ctl::thread;
    /// let allocated = thread::allocatedp::mib().unwrap();
    /// let allocated = allocated.read().unwrap();
    ///
    /// // SAFETY: These reads all occur before thread-local cleanup.
    /// let a = unsafe { allocated.get() };
    /// let buf = vec![0; 1024 * 1024];
    /// let b = unsafe { allocated.get() };
    /// drop(buf);
    /// let c = unsafe { allocated.get() };
    ///
    /// assert!(a < b);
    /// assert_eq!(b, c);
    /// # }
    /// ```
    mib_docs: /// See [`allocatedp`].
}

impl allocatedp {
    /// Reads value using string API.
    pub fn read() -> Result<ThreadLocal<u64>> {
        unsafe { read(Self::name().as_bytes()).map(ThreadLocal) }
    }
}

impl allocatedp_mib {
    /// Reads value using MIB API.
    pub fn read(&self) -> Result<ThreadLocal<u64>> {
        unsafe { read_mib(self.0.as_ref()).map(ThreadLocal) }
    }
}

option! {
    deallocatedp deallocatedp_mib[ name: b"thread.deallocatedp\0", mib: 2 ] => *mut u64 |
    ops:  |
    test: deallocatedp_read_test |
    docs:
    /// Access to the total number of bytes deallocated by the current thread.
    ///
    /// Each call to [`Self::read`] returns a pointer to the calling thread's
    /// counter. [`ThreadLocal::get`] reads it without another `mallctl` call.
    ///
    /// # Example
    ///
    /// ```
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// use tikv_jemalloc_ctl::thread;
    /// let deallocated = thread::deallocatedp::mib().unwrap();
    /// let deallocated = deallocated.read().unwrap();
    ///
    /// // SAFETY: These reads all occur before thread-local cleanup.
    /// let a = unsafe { deallocated.get() };
    /// let buf = vec![0; 1024 * 1024];
    /// let b = unsafe { deallocated.get() };
    /// drop(buf);
    /// let c = unsafe { deallocated.get() };
    ///
    /// assert_eq!(a, b);
    /// assert!(b < c);
    /// # }
    /// ```
    mib_docs: /// See [`deallocatedp`].
}

impl deallocatedp {
    /// Reads value using string API.
    pub fn read() -> Result<ThreadLocal<u64>> {
        unsafe { read(Self::name().as_bytes()).map(ThreadLocal) }
    }
}

impl deallocatedp_mib {
    /// Reads value using MIB API.
    pub fn read(&self) -> Result<ThreadLocal<u64>> {
        unsafe { read_mib(self.0.as_ref()).map(ThreadLocal) }
    }
}

/// A thread-local pointer.
///
/// It is neither `Sync` nor `Send`.
// NB we need *const here specifically since it's !Sync + !Send
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct ThreadLocal<T>(*const T);

impl<T> ThreadLocal<T>
where
    T: Copy,
{
    /// Returns the current value at the pointer.
    ///
    /// # Safety
    ///
    /// The underlying jemalloc thread data must still be live. In a
    /// thread-local destructor, the caller must ensure that jemalloc's
    /// cleanup occurs after this read.
    ///
    /// ```compile_fail
    /// use tikv_jemalloc_ctl::thread::allocatedp;
    ///
    /// let counter = allocatedp::read().unwrap();
    /// let _ = counter.get();
    /// ```
    #[inline]
    pub unsafe fn get(self) -> T {
        unsafe { *self.0 }
    }
}
