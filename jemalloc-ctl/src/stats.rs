//! Global allocator statistics.
//!
//! `jemalloc` tracks a wide variety of statistics. Many of them are cached, and
//! only refreshed when the `jemalloc` "epoch" is advanced. See the [`crate::epoch`] type
//! for more information.

option! {
    allocated[ str: b"stats.allocated\0", non_str: 2 ] => libc::size_t |
    ops: r |
    docs:
    /// Total number of bytes allocated by the application.
    ///
    /// This statistic is cached, and is only refreshed when the epoch is
    /// advanced. See the [`crate::epoch`] type for more information.
    ///
    /// This corresponds to `stats.allocated` in jemalloc's API.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// use tikv_jemalloc_ctl::{epoch, stats};
    /// let e = epoch::mib().unwrap();
    /// let allocated = stats::allocated::mib().unwrap();
    ///
    /// let a = allocated.read().unwrap();
    /// let _buf = vec![0; 1024 * 1024];
    /// e.advance().unwrap();
    /// let b = allocated.read().unwrap();
    /// assert!(a < b);
    /// # }
    /// ```
    mib_docs: /// See [`allocated`].
}

option! {
    active[ str: b"stats.active\0", non_str: 2 ] => libc::size_t |
    ops: r |
    docs:
    /// Total number of bytes in active pages allocated by the application.
    ///
    /// This is a multiple of the page size, and greater than or equal to the
    /// value returned by [`allocated`].
    ///
    /// This statistic is cached, and is only refreshed when the epoch is
    /// advanced. See the [`crate::epoch`] type for more information.
    ///
    /// This corresponds to `stats.active` in jemalloc's API.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// use tikv_jemalloc_ctl::{epoch, stats};
    /// let e = epoch::mib().unwrap();
    /// let active = stats::active::mib().unwrap();
    ///
    /// let a = active.read().unwrap();
    /// let _buf = vec![0; 1024 * 1024];
    /// e.advance().unwrap();
    /// let b = active.read().unwrap();
    /// assert!(a < b);
    /// # }
    /// ```
    mib_docs: /// See [`active`].
}

option! {
    metadata[ str: b"stats.metadata\0", non_str: 2 ] => libc::size_t |
    ops: r |
    docs:
    /// Total number of bytes dedicated to `jemalloc` metadata.
    ///
    /// This statistic is cached, and is only refreshed when the epoch is
    /// advanced. See the [`crate::epoch`] type for more information.
    ///
    /// This corresponds to `stats.metadata` in jemalloc's API.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// use tikv_jemalloc_ctl::{epoch, stats};
    /// let e = epoch::mib().unwrap();
    /// let metadata = stats::metadata::mib().unwrap();
    ///
    /// e.advance().unwrap();
    /// let size = metadata.read().unwrap();
    /// println!("{} bytes of jemalloc metadata", size);
    /// # }
    /// ```
    mib_docs: /// See [`metadata`].
}

option! {
    resident[ str: b"stats.resident\0", non_str: 2 ] => libc::size_t |
    ops: r |
    docs:
    /// Total number of bytes in physically resident data pages mapped by the
    /// allocator.
    ///
    /// This consists of all pages dedicated to allocator metadata, pages
    /// backing active allocations, and unused dirty pages. It may overestimate
    /// the true value because pages may not actually be physically resident if
    /// they correspond to demand-zeroed virtual memory that has not yet been
    /// touched. This is a multiple of the page size, and is larger than the
    /// value returned by [`active`].
    ///
    /// This statistic is cached, and is only refreshed when the epoch is
    /// advanced. See the [`crate::epoch`] type for more information.
    ///
    /// This corresponds to `stats.resident` in jemalloc's API.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// use tikv_jemalloc_ctl::{epoch, stats};
    /// let e = epoch::mib().unwrap();
    /// let resident = stats::resident::mib().unwrap();
    ///
    /// e.advance().unwrap();
    /// let size = resident.read().unwrap();
    /// println!("{} bytes of total resident data", size);
    /// # }
    /// ```
    mib_docs: /// See [`resident`].
}

option! {
    pinned[ str: b"stats.pinned\0", non_str: 2 ] => libc::size_t |
    ops: r |
    docs:
    /// Total number of bytes in *unused* (free) memory extents backed by
    /// non-reclaimable memory, i.e. extents whose allocator hook returned
    /// [`tikv_jemalloc_sys::EXTENT_ALLOC_FLAG_PINNED`]. Extents currently
    /// backing live allocations do not contribute to this value.
    ///
    /// Pinned extents are tracked separately from the other free-extent
    /// categories because they are excluded from decay and purging, so this
    /// statistic stays non-zero only while unused pinned extents remain
    /// cached -- in practice, only when an application installs custom extent
    /// hooks marking non-reclaimable mappings (such as HugeTLB pages) pinned.
    /// Added by the `stats.pinned` mallctl in jemalloc 5.4.0.
    ///
    /// This statistic is cached, and is only refreshed when the epoch is
    /// advanced. See the [`crate::epoch`] type for more information.
    ///
    /// This corresponds to `stats.pinned` in jemalloc's API.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// use tikv_jemalloc_ctl::{epoch, stats};
    /// let e = epoch::mib().unwrap();
    /// let pinned = stats::pinned::mib().unwrap();
    ///
    /// e.advance().unwrap();
    /// let bytes = pinned.read().unwrap();
    /// println!("{} bytes of pinned extents", bytes);
    /// # }
    /// ```
    mib_docs: /// See [`pinned`].
}

option! {
    mapped[ str: b"stats.mapped\0", non_str: 2 ] => libc::size_t |
    ops: r |
    docs:
    /// Total number of bytes in active extents mapped by the allocator.
    ///
    /// This does not include inactive extents, even those that contain unused
    /// dirty pages, so there is no strict ordering between this and the value
    /// returned by [`resident`]. This is a multiple of the page size, and is
    /// larger than the value returned by [`active`].
    ///
    /// This statistic is cached, and is only refreshed when the epoch is
    /// advanced. See the [`crate::epoch`] type for more information.
    ///
    /// This corresponds to `stats.mapped` in jemalloc's API.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// use tikv_jemalloc_ctl::{epoch, stats};
    /// let e = epoch::mib().unwrap();
    /// let mapped = stats::mapped::mib().unwrap();
    ///
    /// e.advance().unwrap();
    /// let size = mapped.read().unwrap();
    /// println!("{} bytes of total mapped data", size);
    /// # }
    /// ```
    mib_docs: /// See [`mapped`].
}

option! {
    retained[ str: b"stats.retained\0", non_str: 2 ] => libc::size_t |
    ops: r |
    docs:
    /// Total number of bytes in virtual memory mappings that were retained
    /// rather than being returned to the operating system via e.g. `munmap(2)`.
    ///
    /// Retained virtual memory is typically untouched, decommitted, or purged,
    /// so it has no strongly associated physical memory. Retained memory is
    /// excluded from mapped memory statistics, e.g. [`mapped`].
    ///
    /// This statistic is cached, and is only refreshed when the epoch is
    /// advanced. See the [`crate::epoch`] type for more information.
    ///
    /// This corresponds to `stats.retained` in jemalloc's API.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// use tikv_jemalloc_ctl::{epoch, stats};
    /// let e = epoch::mib().unwrap();
    /// let retained = stats::retained::mib().unwrap();
    ///
    /// e.advance().unwrap();
    /// let size = retained.read().unwrap();
    /// println!("{} bytes of total retained data", size);
    /// # }
    /// ```
    mib_docs: /// See [`retained`].
}
