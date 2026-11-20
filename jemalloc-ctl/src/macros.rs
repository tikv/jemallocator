//! Utility macros.
//!
//! Naming conventions used by the expansions below (all identifiers derived
//! for a key are spelled out at each call site of `option!`; none are
//! concatenated at compile time):
//!
//! * the companion MIB type is named `<id>_mib`, written directly after the
//!   key identifier: `epoch epoch_mib[ name: ..., mib: 1 ]`;
//! * the generated test is named after the operations:
//!   `<id>_read_write_update_test` for keys supporting `r,w,u` and
//!   `<id>_read_test` otherwise. The `test:` field can use `()` for a key
//!   tested in a separate process.

macro_rules! types {
    ($id:ident $mib_id:ident [ name: $byte_string:expr, mib: $mib_len:expr ] => $ret_ty:ty |
     docs: $(#[$doc:meta])*
     mib_docs: $(#[$doc_mib:meta])*
    ) => {
        $(#[$doc])*
        #[allow(non_camel_case_types)]
        pub struct $id;

        impl $id {
            // SAFETY: Each invocation pairs a jemalloc key with its value type.
            const NAME: crate::keys::Name<'static, $ret_ty> =
                unsafe { crate::keys::Name::new($byte_string) };

            /// Returns Management Information Base (MIB)
            ///
            /// This value can be used to access the key without doing string lookup.
            pub fn mib() -> crate::error::Result<$mib_id> {
                Ok($mib_id(Self::NAME.mib()?))
            }

            /// Key [`crate::keys::Name`].
            pub fn name() -> crate::keys::Name<'static, $ret_ty> {
                Self::NAME
            }
        }

        const _: () = assert!(
            $id::NAME.component_count() == $mib_len,
            concat!(
                "MIB length must match the number of key components for ",
                stringify!($id)
            )
        );

        $(#[$doc_mib])*
        #[repr(transparent)]
        #[derive(Copy, Clone)]
        #[allow(non_camel_case_types)]
        pub struct $mib_id(crate::keys::Mib<$ret_ty, $mib_len>);
    };
}

/// Read
macro_rules! r {
    ($id:ident, $mib_id:ident => $ret_ty:ty) => {
        impl $id {
            /// Reads value using string API.
            pub fn read() -> crate::error::Result<$ret_ty> {
                use crate::keys::Access;
                Self::NAME.read()
            }
        }

        impl $mib_id {
            /// Reads value using MIB API.
            pub fn read(self) -> crate::error::Result<$ret_ty> {
                use crate::keys::Access;
                self.0.read()
            }
        }
    };
}

/// Write
macro_rules! w {
    ($id:ident, $mib_id:ident => $ret_ty:ty) => {
        impl $id {
            /// Writes `value` using string API.
            pub fn write(value: $ret_ty) -> crate::error::Result<()> {
                use crate::keys::Access;
                Self::NAME.write(value)
            }
        }

        impl $mib_id {
            /// Writes `value` using MIB API.
            pub fn write(self, value: $ret_ty) -> crate::error::Result<()> {
                use crate::keys::Access;
                self.0.write(value)
            }
        }
    };
}

/// Update
macro_rules! u {
    ($id:ident, $mib_id:ident => $ret_ty:ty) => {
        impl $id {
            /// Updates the key by name and returns the key-specific result.
            ///
            /// See [`crate::Access::update`] for the return value semantics.
            pub fn update(value: $ret_ty) -> crate::error::Result<$ret_ty> {
                use crate::keys::Access;
                Self::NAME.update(value)
            }
        }

        impl $mib_id {
            /// Updates the key by MIB and returns the key-specific result.
            ///
            /// See [`crate::Access::update`] for the return value semantics.
            pub fn update(
                self,
                value: $ret_ty,
            ) -> crate::error::Result<$ret_ty> {
                use crate::keys::Access;
                self.0.update(value)
            }
        }
    };
}

macro_rules! make_test {
    ($id:ident, $ret_ty:ty, $test_fn:ident, ()) => {};
    ($id:ident, $ret_ty:ty, (), ($($ops:ident),*)) => {};
    (max_background_threads, $ret_ty:ty, $test_fn:ident, ($($ops:ident),+)) => {
        make_test!(max_background_threads, $ret_ty, $test_fn, |_| 1, $($ops),+);
    };
    ($id:ident, $ret_ty:ty, $test_fn:ident, ($($ops:ident),+)) => {
        make_test!($id, $ret_ty, $test_fn, |_| Default::default(), $($ops),+);
    };
    ($id:ident, $ret_ty:ty, $test_fn:ident, $test_val:expr, r, w, u) => {
        #[cfg(test)]
        #[test]
        fn $test_fn() {
            match stringify!($id) {
                "background_thread" |
                "max_background_threads"
                    if cfg!(target_os = "macos") => return,
                // Requires profiling to be enabled at runtime.
                "prof_active" if cfg!(feature = "profiling") => return,
                _ => (),
            }

            let a = $id::read().unwrap();
            let b = $test_val(a);
            let _ = $id::write(b).unwrap();
            let c = $id::read().unwrap();
            assert_eq!(b, c);
            let d = $id::update(a).unwrap();
            let e = $id::read().unwrap();
            assert_eq!(d, c);
            assert_eq!(a, e);

            let mib = $id::mib().unwrap();
            let f = mib.read().unwrap();
            assert_eq!(e, f);
            let g = $test_val(f);
            let _ = mib.write(g).unwrap();
            let h = mib.read().unwrap();
            assert_eq!(g, h);
            let i = mib.update(f).unwrap();
            let j = mib.read().unwrap();
            assert_eq!(i, h);
            assert_eq!(f, j);
        }
    };
    ($id:ident, $ret_ty:ty, $test_fn:ident, $test_val:expr, r) => {
        #[cfg(test)]
        #[test]
        fn $test_fn() {
            let a = $id::read().unwrap();
            let mib = $id::mib().unwrap();
            let b = mib.read().unwrap();
            assert_eq!(a, b);

            #[cfg(feature = "use_std")]
            println!(
                concat!(stringify!($id), " (read): \"{}\" - \"{}\""),
                a, b
            );
        }
    };
}

/// Creates a new option
macro_rules! option {
    ($id:ident $mib_id:ident [ name: $byte_string:expr, mib: $mib_len:expr ] => $ret_ty:ty |
     ops: $($ops:ident),* |
     test: $test_fn:tt |
     docs:
     $(#[$doc:meta])*
     mib_docs:
     $(#[$doc_mib:meta])*
    ) => {
        types! {
            $id $mib_id[ name: $byte_string, mib: $mib_len ] => $ret_ty |
            docs: $(#[$doc])*
            mib_docs: $(#[$doc_mib])*
        }
        $(
            $ops!($id, $mib_id => $ret_ty);
        )*

        make_test!($id, $ret_ty, $test_fn, ($($ops),*));
    };
}
