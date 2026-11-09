//! Per-arena operations.

option! {
    decay_all decay_all_mib[
        str: concat!("arena.", mallctl_arenas_all!(), ".decay\0").as_bytes(),
        non_str: 3
    ] => () |
    ops: c |
    test: decay_all_void_test |
    docs:
    /// Trigger decay-based purging of unused dirty/muzzy pages for all arenas.
    ///
    /// The proportion of unused dirty/muzzy pages to be purged depends on the
    /// current time; see jemalloc's `opt.dirty_decay_ms` and
    /// `opt.muzzy_decay_ms` for details.
    ///
    /// # Example
    ///
    /// ```
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// use tikv_jemalloc_ctl::arena;
    /// arena::decay_all::call().unwrap();
    ///
    /// let decay = arena::decay_all::mib().unwrap();
    /// decay.call().unwrap();
    /// # }
    /// ```
    mib_docs: /// See [`decay_all`].
}

option! {
    purge_all purge_all_mib[
        str: concat!("arena.", mallctl_arenas_all!(), ".purge\0").as_bytes(),
        non_str: 3
    ] => () |
    ops: c |
    test: purge_all_void_test |
    docs:
    /// Purges unused dirty pages of all arenas.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// use tikv_jemalloc_ctl::arena;
    /// arena::purge_all::call().unwrap();
    ///
    /// let purge = arena::purge_all::mib().unwrap();
    /// purge.call().unwrap();
    /// # }
    mib_docs: /// See [`purge_all`].
}
