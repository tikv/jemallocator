//! Utility macros

macro_rules! types {
    ($id:ident[ str: $byte_string:expr, $mib:ty, $name_to_mib:ident ]  |
     docs: $(#[$doc:meta])*
     mib_docs: $(#[$doc_mib:meta])*
    ) => {
        paste::paste! {
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
                pub fn mib() -> crate::error::Result<[<$id _mib>]> {
                    Ok([<$id _mib>](Self::NAME.$name_to_mib()?))
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
            pub struct [<$id _mib>](pub crate::keys::$mib);
        }
    };
}

/// Read
macro_rules! r {
    ($id:ident => $ret_ty:ty) => {
        paste::paste! {
            impl $id {
                /// Reads value using string API.
                pub fn read() -> crate::error::Result<$ret_ty> {
                    use crate::keys::Access;
                    Self::NAME.read()
                }
            }

            impl [<$id _mib>] {
                /// Reads value using MIB API.
                pub fn read(self) -> crate::error::Result<$ret_ty> {
                    use crate::keys::Access;
                    self.0.read()
                }
            }
        }
    };
}

/// Write
macro_rules! w {
    ($id:ident => $ret_ty:ty) => {
        paste::paste! {
            impl $id {
                /// Writes `value` using string API.
                pub fn write(value: $ret_ty) -> crate::error::Result<()> {
                    use crate::keys::Access;
                    Self::NAME.write(value)
                }
            }

            impl [<$id _mib>] {
                /// Writes `value` using MIB API.
                pub fn write(self, value: $ret_ty) -> crate::error::Result<()> {
                    use crate::keys::Access;
                    self.0.write(value)
                }
            }
        }
    };
}

/// Update
macro_rules! u {
    ($id:ident  => $ret_ty:ty) => {
        paste::paste! {
            impl $id {
                /// Updates key to `value` returning its old value using string API.
                pub fn update(value: $ret_ty) -> crate::error::Result<$ret_ty> {
                    use crate::keys::Access;
                    Self::NAME.update(value)
                }
            }

            impl [<$id _mib>] {
                /// Updates key to `value` returning its old value using MIB API.
                pub fn update(self, value: $ret_ty) -> crate::error::Result<$ret_ty> {
                    use crate::keys::Access;
                    self.0.update(value)
                }
            }
        }
    };
}

macro_rules! make_test {
    ($id:ident, $ret_ty:ty, ()) => {};
    (max_background_threads, $ret_ty:ty, ($($ops:ident),+)) => {
        make_test!(max_background_threads, $ret_ty, |_| 1, $($ops),+);
    };
    (epoch, $ret_ty:ty, ($($ops:ident),+)) => {
        make_test!(epoch, $ret_ty, |k| k + 1, $($ops),+);
    };
    ($id:ident, $ret_ty:ty, ($($ops:ident),+)) => {
        make_test!($id, $ret_ty, |_| Default::default(), $($ops),+);
    };
    ($id:ident, $ret_ty:ty, $test_val:expr, r,w,u) => {
        paste::paste! {
            #[cfg(test)]
            #[test]
            fn [<$id _read_write_update_test>]() {
                match stringify!($id) {
                    "background_thread" |
                    "max_background_threads"
                        if cfg!(target_os = "macos") => return,
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
        }
    };
    ($id:ident, $ret_ty:ty, $test_val:expr, r) => {
        paste::paste! {
            #[cfg(test)]
            #[test]
            fn [<$id _read_test>]() {
                let a = $id::read().unwrap();
                let mib = $id::mib().unwrap();
                let b = mib.read().unwrap();
                assert_eq!(a, b);

                #[cfg(feature = "use_std")]
                println!(
                    concat!(
                        stringify!($id),
                        " (read): \"{}\" - \"{}\""),
                    a, b
                );
            }
        }
    };
}

/// Creates a new option
macro_rules! option {
    ($id:ident[ str: $byte_string:expr, $mib:ty, $name_to_mib:ident ] => $ret_ty:ty |
     ops: $($ops:ident),* |
     docs:
     $(#[$doc:meta])*
     mib_docs:
     $(#[$doc_mib:meta])*
    ) => {
        types! {
            $id[ str: $byte_string, $mib, $name_to_mib ] |
            docs: $(#[$doc])*
            mib_docs: $(#[$doc_mib])*
        }
        $(
            $ops!($id => $ret_ty);
        )*

        make_test!($id, $ret_ty, ($($ops),*));
    };
    // Non-string option:
    ($id:ident[ str: $byte_string:expr, non_str: $mib_len:expr ] => $ret_ty:ty |
     ops: $($ops:ident),* |
     docs:
     $(#[$doc:meta])*
     mib_docs:
     $(#[$doc_mib:meta])*
    ) => {
        option! {
            $id[ str: $byte_string, Mib<[usize; $mib_len]>, mib ] => $ret_ty |
            ops: $($ops),* |
            docs: $(#[$doc])*
            mib_docs: $(#[$doc_mib])*
        }
    };
    // String option:
    ($id:ident[ str: $byte_string:expr, str: $mib_len:expr ] => $ret_ty:ty |
     ops: $($ops:ident),* |
     docs:
     $(#[$doc:meta])*
     mib_docs:
     $(#[$doc_mib:meta])*
    ) => {
        option! {
            $id[ str: $byte_string, MibStr<[usize; $mib_len]>, mib_str ] => $ret_ty |
            ops: $($ops),* |
            docs: $(#[$doc])*
            mib_docs: $(#[$doc_mib])*
        }
    };
}
