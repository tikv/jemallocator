//! Utility macros.
//!
//! Naming conventions used by the expansions below (all identifiers derived
//! for a key are spelled out at each call site of `option!`; none are
//! concatenated at compile time):
//!
//! * the companion MIB type is named `<id>_mib`, written directly after the
//!   key identifier: `epoch epoch_mib[ str: ..., non_str: 1 ]`;
//! * the generated test is named after the operations:
//!   `<id>_read_write_update_test` for keys supporting `r,w,u` and
//!   `<id>_read_test` otherwise; it is passed via the `test:` field.

macro_rules! types {
    ($id:ident $mib_id:ident [ str: $byte_string:expr, $mib_ty:ty, $name_to_mib:ident ] |
     docs: $(#[$doc:meta])*
     mib_docs: $(#[$doc_mib:meta])*
    ) => {
        $(#[$doc])*
        #[allow(non_camel_case_types)]
        pub struct $id;

        impl $id {
            const NAME: &'static crate::keys::Name = {
                union U<'a> {
                    bytes: &'a [u8],
                    name: &'a crate::keys::Name
                }

                unsafe { U { bytes: $byte_string }.name }
            };
            /// Returns Management Information Base (MIB)
            ///
            /// This value can be used to access the key without doing string lookup.
            pub fn mib() -> crate::error::Result<$mib_id> {
                Ok($mib_id(Self::NAME.$name_to_mib()?))
            }

            /// Key [`crate::keys::Name`].
            pub fn name() -> &'static crate::keys::Name {
                Self::NAME
            }
        }

        $(#[$doc_mib])*
        #[repr(transparent)]
        #[derive(Copy, Clone)]
        #[allow(non_camel_case_types)]
        pub struct $mib_id(pub $mib_ty);
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
            /// Updates key to `value` returning its old value using string API.
            pub fn update(value: $ret_ty) -> crate::error::Result<$ret_ty> {
                use crate::keys::Access;
                Self::NAME.update(value)
            }
        }

        impl $mib_id {
            /// Updates key to `value` returning its old value using MIB API.
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
    (max_background_threads, $ret_ty:ty, $test_fn:ident, ($($ops:ident),+)) => {
        make_test!(max_background_threads, $ret_ty, $test_fn, |_| 1, $($ops),+);
    };
    (epoch, $ret_ty:ty, $test_fn:ident, ($($ops:ident),+)) => {
        make_test!(epoch, $ret_ty, $test_fn, |k| k + 1, $($ops),+);
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
                // Requires `opt.prof` and can race with hook tests.
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
            if stringify!($id) == "epoch" {
                assert_eq!(d, e);
                assert_ne!(a, e);
            } else {
                assert_eq!(d, c);
                assert_eq!(a, e);
            }

            let mib = $id::mib().unwrap();
            let f = mib.read().unwrap();
            assert_eq!(e, f);
            let g = $test_val(f);
            let _ = mib.write(g).unwrap();
            let h = mib.read().unwrap();
            assert_eq!(g, h);
            let i = mib.update(f).unwrap();
            let j = mib.read().unwrap();
            if stringify!($id) == "epoch" {
                assert_eq!(i, j);
                assert_ne!(f, j);
            } else {
                assert_eq!(i, h);
                assert_eq!(f, j);
            }
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
    ($id:ident $mib_id:ident [ str: $byte_string:expr, $mib_ty:ty, $name_to_mib:ident ] => $ret_ty:ty |
     ops: $($ops:ident),* |
     test: $test_fn:ident |
     docs:
     $(#[$doc:meta])*
     mib_docs:
     $(#[$doc_mib:meta])*
    ) => {
        types! {
            $id $mib_id[ str: $byte_string, $mib_ty, $name_to_mib ] |
            docs: $(#[$doc])*
            mib_docs: $(#[$doc_mib])*
        }
        $(
            $ops!($id, $mib_id => $ret_ty);
        )*

        make_test!($id, $ret_ty, $test_fn, ($($ops),*));
    };
    // Non-string option:
    ($id:ident $mib_id:ident [ str: $byte_string:expr, non_str: $mib_len:expr ] => $ret_ty:ty |
     ops: $($ops:ident),* |
     test: $test_fn:ident |
     docs:
     $(#[$doc:meta])*
     mib_docs:
     $(#[$doc_mib:meta])*
    ) => {
        option! {
            $id $mib_id[ str: $byte_string, crate::keys::Mib<[usize; $mib_len]>, mib ] => $ret_ty |
            ops: $($ops),* |
            test: $test_fn |
            docs: $(#[$doc])*
            mib_docs: $(#[$doc_mib])*
        }
    };
    // String option:
    ($id:ident $mib_id:ident [ str: $byte_string:expr, str: $mib_len:expr ] => $ret_ty:ty |
     ops: $($ops:ident),* |
     test: $test_fn:ident |
     docs:
     $(#[$doc:meta])*
     mib_docs:
     $(#[$doc_mib:meta])*
    ) => {
        option! {
            $id $mib_id[ str: $byte_string, crate::keys::MibStr<[usize; $mib_len]>, mib_str ] => $ret_ty |
            ops: $($ops),* |
            test: $test_fn |
            docs: $(#[$doc])*
            mib_docs: $(#[$doc_mib])*
        }
    };
}
