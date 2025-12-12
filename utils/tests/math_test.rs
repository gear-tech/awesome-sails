// This file is part of Gear.

// Copyright (C) 2025 Gear Technologies Inc.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use awesome_sails_utils::{impl_math_wrapper, math::*};
use proptest::prelude::*;
use sails_rs::{Decode, Encode, TypeInfo, U256};

// UPDATED: Now defining types by BYTE count, not bits/limbs.
type Uint64 = CustomUint<8>; // 64 bits = 8 bytes
type Uint72 = CustomUint<9>; // 72 bits = 9 bytes
type Uint80 = CustomUint<10>; // 80 bits = 10 bytes
type Uint128 = CustomUint<16>; // 128 bits = 16 bytes
type Uint256 = CustomUint<32>; // 256 bits = 32 bytes
type Uint512 = CustomUint<64>; // 512 bits = 64 bytes
type Uint1024 = CustomUint<128>; // 1024 bits = 128 bytes

fn any_u256() -> impl Strategy<Value = U256> {
    proptest::collection::vec(any::<u8>(), 32).prop_map(|bytes| U256::from_little_endian(&bytes))
}

macro_rules! test_primitive_math {
    ($(($t:ty, $mod_name:ident)),*) => {
        $(
            mod $mod_name {
                #![allow(non_snake_case)]
                use super::*;

                #[test]
                fn test_traits_presence() {
                    let max = <$t>::MAX;
                    let min = <$t>::MIN;
                    let one = <$t>::ONE;
                    let zero = <$t>::ZERO;

                    assert!(max.is_max());
                    assert!(min.is_min());
                    assert!(one.is_one());
                    assert!(zero.is_zero());
                }

                #[test]
                fn test_checked_math_edges() {
                    let max = <$t>::MAX;
                    let min = <$t>::MIN;
                    let one = <$t>::ONE;
                    let zero = <$t>::ZERO;

                    // 1. Option-based checks (Standard)
                    // Overflow
                    assert_eq!(max.checked_add(one), None, "MAX + 1 should return None");
                    // Underflow
                    assert_eq!(min.checked_sub(one), None, "MIN - 1 should return None");

                    // 2. Result-based checks (Error types)
                    // OverflowError
                    assert_eq!(max.checked_add_err(one), Err(OverflowError), "MAX + 1 should return OverflowError");
                    // UnderflowError
                    assert_eq!(min.checked_sub_err(one), Err(UnderflowError), "MIN - 1 should return UnderflowError");

                    // 3. Valid operations
                    assert_eq!(max.checked_sub(one), Some(max - one));
                    assert_eq!(min.checked_add(one), Some(min + one));

                    // 4. Identity
                    assert_eq!(max.checked_add(zero), Some(max));
                    assert_eq!(min.checked_sub(zero), Some(min));
                }

                // Helper trait to unify overflow signatures for testing
                trait TestOverflow: Sized + CheckedMath + Copy {
                    fn test_overflowing_add(self, rhs: Self) -> (Self, bool);
                    fn test_overflowing_sub(self, rhs: Self) -> (Self, bool);
                }

                impl TestOverflow for $t {
                    fn test_overflowing_add(self, rhs: Self) -> (Self, bool) { self.overflowing_add(rhs) }
                    fn test_overflowing_sub(self, rhs: Self) -> (Self, bool) { self.overflowing_sub(rhs) }
                }

                proptest! {
                    #[test]
                    fn fuzz_checked_add(a in any::<$t>(), b in any::<$t>()) {
                        let res_opt = a.checked_add(b);
                        let res_err = a.checked_add_err(b);
                        let (expected, overflow) = a.test_overflowing_add(b);

                        if overflow {
                            prop_assert_eq!(res_opt, None);
                            prop_assert_eq!(res_err, Err(OverflowError));
                        } else {
                            prop_assert_eq!(res_opt, Some(expected));
                            prop_assert_eq!(res_err, Ok(expected));
                        }
                    }

                    #[test]
                    fn fuzz_checked_sub(a in any::<$t>(), b in any::<$t>()) {
                        let res_opt = a.checked_sub(b);
                        let res_err = a.checked_sub_err(b);
                        let (expected, overflow) = a.test_overflowing_sub(b);

                        if overflow {
                            prop_assert_eq!(res_opt, None);
                            prop_assert_eq!(res_err, Err(UnderflowError));
                        } else {
                            prop_assert_eq!(res_opt, Some(expected));
                            prop_assert_eq!(res_err, Ok(expected));
                        }
                    }
                }
            }
        )*
    }
}

test_primitive_math!(
    (u8, mod_u8),
    (u16, mod_u16),
    (u32, mod_u32),
    (u64, mod_u64),
    (u128, mod_u128),
    (usize, mod_usize),
    (i8, mod_i8),
    (i16, mod_i16),
    (i32, mod_i32),
    (i64, mod_i64),
    (i128, mod_i128),
    (isize, mod_isize)
);

// Special case for U256 as it doesn't have standard overflowing_* methods in the same trait hierarchy
mod mod_u256 {
    use super::*;

    #[test]
    fn test_edges() {
        let max = U256::MAX;
        let one = U256::one();

        // Option
        assert_eq!(max.checked_add(one), None);
        assert_eq!(U256::zero().checked_sub(one), None);

        // Result
        assert_eq!(max.checked_add_err(one), Err(OverflowError));
        assert_eq!(U256::zero().checked_sub_err(one), Err(UnderflowError));
    }

    proptest! {
        #[test]
        fn fuzz_add(a in any_u256(), b in any_u256()) {
            let res = a.checked_add(b);
            let (expected, overflow) = a.overflowing_add(b);
            if overflow {
                prop_assert!(res.is_none());
            } else {
                prop_assert_eq!(res.unwrap(), expected);
            }
        }
    }
}

mod custom_uint {
    use super::*;

    mod codec {
        use super::*;

        proptest! {
            #[test]
            fn test_uint72_compact_encoding(val in any::<u64>(), high_byte in 0u8..=0xFF) {
                // Manually construct a 72-bit value
                let mut bytes = [0u8; 32];
                bytes[..8].copy_from_slice(&val.to_le_bytes());
                bytes[8] = high_byte;
                let u256_val = U256::from_little_endian(&bytes);

                let num = Uint72::try_from(u256_val).unwrap();
                let encoded = num.encode();

                // Must be exactly 9 bytes (72 bits)
                prop_assert_eq!(encoded.len(), 9);
                prop_assert_eq!(encoded[8], high_byte);

                // Roundtrip
                let decoded = Uint72::decode(&mut &encoded[..]).unwrap();
                prop_assert_eq!(num, decoded);
            }

            #[test]
            fn test_uint80_roundtrip(bytes in proptest::collection::vec(any::<u8>(), 10)) {
                let decoded = Uint80::decode(&mut &bytes[..]).unwrap();
                let encoded = decoded.encode();
                prop_assert_eq!(encoded, bytes);
            }

            #[test]
            fn test_uint512_roundtrip(bytes in proptest::collection::vec(any::<u8>(), 64)) {
                // 512 bits = 64 bytes
                let decoded = Uint512::decode(&mut &bytes[..]).unwrap();
                let encoded = decoded.encode();
                prop_assert_eq!(encoded, bytes);
            }

            #[test]
            fn test_uint1024_roundtrip(bytes in proptest::collection::vec(any::<u8>(), 128)) {
                let decoded = Uint1024::decode(&mut &bytes[..]).unwrap();
                let encoded = decoded.encode();
                prop_assert_eq!(encoded, bytes);
            }
        }

        #[test]
        fn test_exact_sizes() {
            // This test ensures we check all defined aliases, avoiding unused warnings
            assert_eq!(Uint64::default().encode().len(), 8);
            assert_eq!(Uint72::default().encode().len(), 9);
            assert_eq!(Uint80::default().encode().len(), 10);
            assert_eq!(Uint128::default().encode().len(), 16);
            assert_eq!(Uint256::default().encode().len(), 32);
            assert_eq!(Uint512::default().encode().len(), 64);
            assert_eq!(Uint1024::default().encode().len(), 128);
        }
    }

    mod conversions {
        use super::*;

        proptest! {
            #[test]
            fn test_try_resize_up(val in any::<u64>()) {
                let small = Uint64::from(val);
                // 64 -> 256
                let big: Uint256 = small.try_resize().unwrap();
                // 256 -> 64
                let back: Uint64 = big.try_resize().unwrap();
                prop_assert_eq!(small, back);
            }

            #[test]
            fn test_try_resize_down_overflow(val_bytes in proptest::collection::vec(any::<u8>(), 32)) {
                let big = Uint256::decode(&mut &val_bytes[..]).unwrap();
                // Check if any byte beyond the 8th (index 7) is non-zero
                let needs_more_than_64 = val_bytes[8..].iter().any(|&b| b != 0);

                let res: Result<Uint64, _> = big.try_resize();
                if needs_more_than_64 {
                    prop_assert!(res.is_err());
                } else {
                    prop_assert!(res.is_ok());
                }
            }

            #[test]
            fn test_u128_conversions(val in any::<u128>()) {
                // TryFrom u128 -> CustomUint
                let num = Uint128::try_from(val).unwrap();

                // TryFrom CustomUint -> u128
                let back: u128 = num.try_into().unwrap();
                prop_assert_eq!(back, val);

                // Overflow check for smaller int (Uint64)
                if val > u64::MAX as u128 {
                    prop_assert!(Uint64::try_from(val).is_err());
                }
            }

            #[test]
            fn test_u256_conversions(val in any_u256()) {
                let num = Uint256::try_from(val).unwrap();
                let back: U256 = num.try_into().unwrap();
                prop_assert_eq!(back, val);
            }
        }
    }

    mod nonzero {
        use super::*;

        #[test]
        fn test_basic_logic() {
            assert!(NonZero::try_new(Uint64::ZERO).is_err());
            let one = NonZero::try_new(Uint64::ONE).unwrap();
            let two = one.try_add(one).unwrap();

            assert_eq!(two.into_inner(), Uint64::from(2));
            assert!(one < two);
        }

        #[test]
        fn test_casting() {
            let one = NonZero::try_new(Uint64::ONE).unwrap();

            // try_cast: NonZero<T> -> Result<U, ...>
            let as_u64: u64 = one.try_cast().unwrap();
            assert_eq!(as_u64, 1);

            // cast: NonZero<T> -> U (requires From)
            let as_custom: Uint64 = one.cast();
            assert_eq!(as_custom, Uint64::ONE);
        }
    }
}

mod wrappers {
    use super::*;

    /// Test the macro in DEFAULT mode (2 arguments).
    /// Should generate `TryFrom` implementations automatically.
    mod default_mode {
        use super::*;

        #[derive(
            Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo,
        )]
        #[codec(crate = sails_rs::scale_codec)]
        #[scale_info(crate = sails_rs::scale_info)]
        struct WrapperDefault(u64);

        impl_math_wrapper!(WrapperDefault, u64);

        #[test]
        fn test_auto_try_from() {
            let w = WrapperDefault(100);

            // TryFrom for u128 should be generated
            let as_u128: u128 = w.try_into().unwrap();
            assert_eq!(as_u128, 100);

            // TryFrom for U256 should be generated
            let as_u256: U256 = w.try_into().unwrap();
            assert_eq!(as_u256, U256::from(100));
        }

        #[test]
        fn test_math_ops() {
            let a = WrapperDefault(10);
            let b = WrapperDefault(20);
            assert_eq!(a.checked_add(b).unwrap().0, 30);

            // Check specific error type
            assert_eq!(
                WrapperDefault::MAX.checked_add_err(WrapperDefault(1)),
                Err(OverflowError)
            );
        }

        #[test]
        fn test_comparisons_inner() {
            let a = WrapperDefault(10);
            // assert_eq! requires same types, so we use assert!(==) for mixed types
            // or we must verify PartialEq<u64> works
            assert!(a == 10u64);
            assert!(a < 20u64);
            assert!(a != 20u64);
        }
    }

    /// Test the macro in MANUAL mode (3 arguments).
    /// Should NOT generate `TryFrom` for u128/U256, allowing manual `From`.
    mod manual_mode {
        use super::*;

        #[derive(
            Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo,
        )]
        #[codec(crate = sails_rs::scale_codec)]
        #[scale_info(crate = sails_rs::scale_info)]
        struct WrapperManual(u64);

        // Use flag `manual_from`
        impl_math_wrapper!(WrapperManual, u64, manual_from);

        // Manual implementation of From (which forbids auto TryFrom generation in macro)
        impl From<WrapperManual> for u128 {
            fn from(w: WrapperManual) -> u128 {
                w.0 as u128
            }
        }

        // Manual implementation for U256
        impl From<WrapperManual> for U256 {
            fn from(w: WrapperManual) -> U256 {
                U256::from(w.0)
            }
        }

        #[test]
        fn test_manual_from_works() {
            let w = WrapperManual(42);

            // Since we implemented From, .into() should work (infallible)
            let as_u128: u128 = w.into();
            assert_eq!(as_u128, 42);

            let as_u256: U256 = w.into();
            assert_eq!(as_u256, U256::from(42));
        }

        #[test]
        fn test_non_zero_integration() {
            // Check that From<NonZero> logic still works
            let one = NonZero::try_new(WrapperManual(1)).unwrap();
            let back: WrapperManual = one.into();
            assert_eq!(back.0, 1);
        }
    }
}
