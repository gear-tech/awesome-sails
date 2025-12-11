use awesome_sails_utils::math::*; // Import everything from math.rs
use proptest::prelude::*;
use sails_rs::{Decode, Encode};

// Common types for testing
type Uint64 = CustomUint<64, 1>;
type Uint128 = CustomUint<128, 2>;
type Uint256 = CustomUint<256, 4>;
type Uint512 = CustomUint<512, 8>;

// Macro to generate tests for primitive types ensuring Min/Max coverage + Fuzzing
macro_rules! test_primitive_math {
    ($(($t:ty, $mod_name:ident)),*) => {
        $(
            mod $mod_name { // Module per type to avoid name collisions
                #![allow(non_snake_case)]
                use super::*;

                #[test]
                fn test_edge_cases() {
                    let max = <$t>::MAX;
                    let min = <$t>::MIN;
                    let one = <$t>::ONE;
                    let zero = <$t>::ZERO;

                    // Overflow
                    assert_eq!(max.checked_add(one), None, "MAX + 1 should overflow");

                    // Underflow
                    assert_eq!(min.checked_sub(one), None, "MIN - 1 should underflow");

                    // Valid operations near edges
                    assert_eq!(max.checked_sub(one), Some(max - one), "MAX - 1 should be valid");
                    assert_eq!(min.checked_add(one), Some(min + one), "MIN + 1 should be valid");

                    // Identity
                    assert_eq!(max.checked_add(zero), Some(max));
                    assert_eq!(min.checked_sub(zero), Some(min));

                    // Self operations
                    assert_eq!(max.checked_sub(max), Some(zero));
                    assert_eq!(min.checked_sub(min), Some(zero)); // For signed types MIN-MIN=0

                    // Small numbers
                    if max > one { // Ensure type > 1-bit
                         let two = one + one;
                         assert_eq!(one.checked_add(one), Some(two));
                         assert_eq!(one.checked_sub(one), Some(zero));
                    }
                }

                // We need a helper trait because U256 overflowing_add signature might differ or not be standard
                trait TestOverflow: Sized + CheckedMath + Copy {
                    fn test_overflowing_add(self, rhs: Self) -> (Self, bool);
                    fn test_overflowing_sub(self, rhs: Self) -> (Self, bool);
                }

                // Implement for primitives
                impl TestOverflow for $t {
                    fn test_overflowing_add(self, rhs: Self) -> (Self, bool) {
                        self.overflowing_add(rhs)
                    }
                    fn test_overflowing_sub(self, rhs: Self) -> (Self, bool) {
                        self.overflowing_sub(rhs)
                    }
                }

                proptest! {
                    #[test]
                    fn test_fuzz_add(a in any::<$t>(), b in any::<$t>()) {
                        let res: Option<$t> = a.checked_add(b);
                        let (expected, overflow) = a.test_overflowing_add(b);
                        if overflow {
                            prop_assert!(res.is_none());
                        } else {
                            prop_assert_eq!(res.unwrap(), expected);
                        }
                    }

                    #[test]
                    fn test_fuzz_sub(a in any::<$t>(), b in any::<$t>()) {
                        let res: Option<$t> = a.checked_sub(b);
                        let (expected, overflow) = a.test_overflowing_sub(b);
                        if overflow {
                            prop_assert!(res.is_none());
                        } else {
                            prop_assert_eq!(res.unwrap(), expected);
                        }
                    }
                }
            }
        )*
    }
}

// Generate tests for all primitives supported by impl_checked_math!
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

// Manual test module for U256 (no Arbitrary impl)
mod mod_u256 {
    use super::*;

    #[test]
    fn test_edge_cases() {
        let max = U256::MAX;
        let min = U256::MIN;
        let one = U256::ONE;
        let zero = U256::ZERO;

        // Overflow
        assert_eq!(max.checked_add(one), None);

        // Underflow
        assert_eq!(min.checked_sub(one), None);

        // Valid operations near edges
        assert_eq!(max.checked_sub(one), Some(max - one));
        assert_eq!(min.checked_add(one), Some(min + one));

        // Identity
        assert_eq!(max.checked_add(zero), Some(max));
        assert_eq!(min.checked_sub(zero), Some(min));

        // Self operations
        assert_eq!(max.checked_sub(max), Some(zero));
        assert_eq!(min.checked_sub(min), Some(zero));

        // Small numbers
        let two = one + one;
        assert_eq!(one.checked_add(one), Some(two));
        assert_eq!(one.checked_sub(one), Some(zero));
    }

    // Strategy to generate U256
    fn any_u256() -> impl Strategy<Value = U256> {
        proptest::collection::vec(any::<u8>(), 32)
            .prop_map(|bytes| U256::from_little_endian(&bytes))
    }

    proptest! {
        #[test]
        fn test_fuzz_add(a in any_u256(), b in any_u256()) {
            let res = a.checked_add(b);
            let (expected, overflow) = a.overflowing_add(b);
            if overflow {
                prop_assert!(res.is_none());
            } else {
                prop_assert_eq!(res.unwrap(), expected);
            }
        }

        #[test]
        fn test_fuzz_sub(a in any_u256(), b in any_u256()) {
            let res = a.checked_sub(b);
            let (expected, overflow) = a.overflowing_sub(b);
            if overflow {
                prop_assert!(res.is_none());
            } else {
                prop_assert_eq!(res.unwrap(), expected);
            }
        }
    }
}

mod codec {
    use super::*;

    proptest! {
        #[test]
        fn test_uint72_roundtrip(val in any::<u64>(), high_byte in 0u8..=0xFF) {
            // Construct a 72-bit value: 64 bits from `val` + 8 bits from `high_byte`.
            let mut bytes = [0u8; 32];
            bytes[..8].copy_from_slice(&val.to_le_bytes());
            bytes[8] = high_byte;
            let u256_val = U256::from_little_endian(&bytes);

            type Uint72 = CustomUint<72, 2>;
            let num = Uint72::try_from(u256_val).unwrap();
            let encoded = num.encode();

            prop_assert_eq!(encoded.len(), 9, "Uint72 should encode to 9 bytes");
            prop_assert_eq!(encoded[8], high_byte, "9th byte must match");

            let decoded = Uint72::decode(&mut &encoded[..]).unwrap();
            prop_assert_eq!(num, decoded, "Decoded value must match original");
        }

        #[test]
        fn test_uint256_roundtrip(bytes in proptest::collection::vec(any::<u8>(), 32)) {
            let u256_val = U256::from_little_endian(&bytes);
            let num = Uint256::try_from(u256_val).unwrap();

            let encoded = num.encode();
            prop_assert_eq!(encoded.len(), 32);

            let decoded = Uint256::decode(&mut &encoded[..]).unwrap();
            prop_assert_eq!(num, decoded);
        }

        #[test]
        fn test_uint512_roundtrip(bytes in proptest::collection::vec(any::<u8>(), 64)) {
            let decoded = Uint512::decode(&mut &bytes[..]).unwrap();
            let encoded = decoded.encode();

            prop_assert_eq!(encoded.len(), 64);
            prop_assert_eq!(encoded, bytes, "Encoded bytes should match original random bytes for 512-bit");
        }

        #[test]
        fn test_uint1024_roundtrip(bytes in proptest::collection::vec(any::<u8>(), 128)) {
            // 1024 bits = 128 bytes.
            // 1024 / 64 = 16 limbs.
            type Uint1024 = CustomUint<1024, 16>;

            let decoded = Uint1024::decode(&mut &bytes[..]).unwrap();
            let encoded = decoded.encode();

            prop_assert_eq!(encoded.len(), 128);
            prop_assert_eq!(encoded, bytes, "Encoded bytes should match original random bytes for 1024-bit");
        }
    }

    #[test]
    fn test_exact_size_encoding() {
        type Uint72 = CustomUint<72, 2>;
        // Test 72 bits = 9 bytes
        let val = Uint72::ONE;
        assert_eq!(val.encode().len(), 9);

        // Test 128 bits = 16 bytes
        let val = Uint128::ONE;
        assert_eq!(val.encode().len(), 16);
    }

    #[test]
    fn test_decode_eof_errors() {
        type Uint72 = CustomUint<72, 2>;
        // 72 bits needs 9 bytes. Provide 8.
        let bytes = [0u8; 8];
        let res = Uint72::decode(&mut &bytes[..]);
        assert!(res.is_err());
    }
}

mod ops {
    use super::*;

    proptest! {
        // --- CONVERSIONS ---

        #[test]
        fn test_u128_conversion(val in any::<u128>()) {
            let num = Uint128::try_from(val).unwrap();
            prop_assert_eq!(num.into_inner(), ruint::Uint::<128, 2>::from(val));

            let back: u128 = num.try_into().unwrap();
            prop_assert_eq!(back, val);
        }

        #[test]
        fn test_u128_overflow(val in any::<u128>()) {
            // Uint64 cannot hold u128 (unless value fits in u64)
            if val > u64::MAX as u128 {
                prop_assert!(Uint64::try_from(val).is_err());
            } else {
                prop_assert!(Uint64::try_from(val).is_ok());
            }
        }

        #[test]
        fn test_u256_conversion(val_bytes in proptest::collection::vec(any::<u8>(), 32)) {
            let val = U256::from_little_endian(&val_bytes);
            let num = Uint256::try_from(val).unwrap();
            let back: U256 = num.try_into().unwrap();
            prop_assert_eq!(val, back);
        }

        #[test]
        fn test_u256_overflow(val_bytes in proptest::collection::vec(any::<u8>(), 32)) {
            let val = U256::from_little_endian(&val_bytes);
            // Uint128 cannot hold U256 if > u128::MAX
            if val > U256::from(u128::MAX) {
                prop_assert!(Uint128::try_from(val).is_err());
            }
        }

        // --- RESIZE ---

        #[test]
        fn test_resize_up(val in any::<u64>()) {
            let small = Uint64::from(val);
            let big: Uint256 = small.try_resize().unwrap();
            let back: Uint64 = big.try_resize().unwrap();
            prop_assert_eq!(small, back);
        }

        #[test]
        fn test_resize_down_ok(val in any::<u64>()) {
            let big = Uint256::from(val);
            let small: Uint64 = big.try_resize().unwrap();
            prop_assert_eq!(Uint64::from(val), small);
        }

        #[test]
        fn test_resize_down_fail(val_bytes in proptest::collection::vec(any::<u8>(), 32)) {
            let big = Uint256::decode(&mut &val_bytes[..]).unwrap();
            // If any byte beyond 8th is non-zero, resize to 64-bit must fail
            let needs_more_than_64 = val_bytes[8..].iter().any(|&b| b != 0);

            let res: Result<Uint64, _> = big.try_resize();
            if needs_more_than_64 {
                prop_assert!(res.is_err());
            } else {
                prop_assert!(res.is_ok());
            }
        }

        // --- MATH (CustomUint) ---

        #[test]
        fn test_add_overflow(a in any::<u64>(), b in any::<u64>()) {
            let ua = Uint64::from(a);
            let ub = Uint64::from(b);
            let sum = ua.checked_add(ub);

            let (expected, overflow) = a.overflowing_add(b);
            if overflow {
                prop_assert!(sum.is_none());
            } else {
                let res: u128 = sum.unwrap().try_into().unwrap();
                prop_assert_eq!(res, expected as u128);
            }
        }

        #[test]
        fn test_sub_underflow(a in any::<u64>(), b in any::<u64>()) {
            let ua = Uint64::from(a);
            let ub = Uint64::from(b);
            let diff = ua.checked_sub(ub);

            let (expected, underflow) = a.overflowing_sub(b);
            if underflow {
                prop_assert!(diff.is_none());
            } else {
                let res: u128 = diff.unwrap().try_into().unwrap();
                prop_assert_eq!(res, expected as u128);
            }
        }
    }

    #[test]
    fn test_nonzero() {
        // 0 -> Err
        assert!(NonZero::try_new(Uint64::ZERO).is_err());
        // 1 -> Ok
        assert!(NonZero::try_new(Uint64::ONE).is_ok());

        let one = NonZero::try_new(Uint64::ONE).unwrap();
        let two = one.try_add(one).unwrap();
        assert_eq!(two.into_inner(), Uint64::from(2));

        // Test try_sub
        let three = NonZero::try_new(Uint64::from(3)).unwrap();
        let diff = three.try_sub(one).unwrap();
        assert_eq!(diff.into_inner(), Uint64::from(2));
        assert!(one.try_sub(three).is_err()); // Underflow: one - three should fail

        // Test try_cast to u64 (should succeed for Uint64)
        let try_casted_one_u64: Result<u64, _> = one.try_cast();
        assert_eq!(try_casted_one_u64.unwrap(), 1u64);

        // Test try_cast to u128 (should succeed for Uint64)
        let try_casted_one_u128: Result<u128, _> = one.try_cast();
        assert_eq!(try_casted_one_u128.unwrap(), 1u128);

        // Test try_cast to u64 with overflow (using Uint128)
        let big_uint128 = NonZero::try_new(Uint128::MAX).unwrap();
        let try_cast_overflow_u64: Result<u64, _> = big_uint128.try_cast();
        assert!(try_cast_overflow_u64.is_err());

        // Test PartialEq (NonZero<T> vs NonZero<T>)
        let another_one = NonZero::try_new(Uint64::ONE).unwrap();
        assert_eq!(one, another_one);
        assert_ne!(one, two);

        // Test Ord/PartialOrd (NonZero<T> vs NonZero<T>)
        assert!(one < two);
        assert!(two > one);
        assert!(one <= one);
        assert!(one >= one);
    }
}

mod math_wrapper_tests {
    use super::*;
    use awesome_sails_utils::{
        impl_math_wrapper,
        math::{CheckedMath, Max, Min, One, Zero},
    };

    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Default,
        sails_rs::Decode,
        sails_rs::Encode,
        sails_rs::TypeInfo,
    )]
    #[codec(crate = sails_rs::scale_codec)]
    #[scale_info(crate = sails_rs::scale_info)]
    struct TestWrapper(u64);

    // Apply the macro for our TestWrapper
    impl_math_wrapper!(TestWrapper, u64);

    // Implement Arbitrary for TestWrapper to enable proptest fuzzing
    impl Arbitrary for TestWrapper {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            any::<u64>().prop_map(TestWrapper).boxed()
        }
    }

    #[test]
    fn test_wrapper_constants() {
        assert_eq!(TestWrapper::MAX.0, u64::MAX);
        assert_eq!(TestWrapper::MIN.0, u64::MIN);
        assert_eq!(TestWrapper::ONE.0, 1);
        assert_eq!(TestWrapper::ZERO.0, 0);
    }

    proptest! {
        #[test]
        fn test_wrapper_checked_add(a in any::<u64>(), b in any::<u64>()) {
            let wa = TestWrapper(a);
            let wb = TestWrapper(b);

            let res = wa.checked_add(wb);
            let expected_res = a.checked_add(b);

            prop_assert_eq!(res.map(|w| w.0), expected_res);
        }

        #[test]
        fn test_wrapper_checked_sub(a in any::<u64>(), b in any::<u64>()) {
            let wa = TestWrapper(a);
            let wb = TestWrapper(b);

            let res = wa.checked_sub(wb);
            let expected_res = a.checked_sub(b);

            prop_assert_eq!(res.map(|w| w.0), expected_res);
        }
    }

    #[test]
    fn test_wrapper_nonzero_conversion() {
        // Test try_from for non-zero
        let one_wrapper = TestWrapper(1);
        let non_zero_one = NonZero::try_from(one_wrapper).unwrap();
        assert_eq!(non_zero_one.into_inner().0, 1);

        // Test try_from for zero (should fail)
        let zero_wrapper = TestWrapper(0);
        assert!(NonZero::try_from(zero_wrapper).is_err());

        // Test From<NonZero> for wrapper
        let back_to_wrapper: TestWrapper = non_zero_one.into();
        assert_eq!(back_to_wrapper.0, 1);
    }

    proptest! {
        #[test]
        fn test_wrapper_u256_conversion(val_bytes in proptest::collection::vec(any::<u8>(), 32)) {
            let u256_val = U256::from_little_endian(&val_bytes);
            let wrapper = TestWrapper::try_from(u256_val);

            // Test case where u256_val fits into u64
            if u256_val <= U256::from(u64::MAX) {
                let unwrapped = wrapper.unwrap();
                prop_assert_eq!(unwrapped.0, u256_val.as_u64());

                let back_to_u256: U256 = unwrapped.into();
                prop_assert_eq!(back_to_u256, u256_val);
            } else {
                // Test case where u256_val does not fit into u64
                prop_assert!(wrapper.is_err());
            }
        }
    }

    proptest! {
        #[test]
        fn test_wrapper_u128_conversion(val in any::<u128>()) {
            let wrapper = TestWrapper::try_from(val);

            // Test case where u128_val fits into u64
            if val <= u64::MAX as u128 {
                let unwrapped = wrapper.unwrap();
                prop_assert_eq!(unwrapped.0, val as u64);

                let back_to_u128: u128 = unwrapped.into();
                prop_assert_eq!(back_to_u128, val);
            } else {
            }
        }
    }

    #[test]
    fn test_wrapper_partial_eq() {
        let a = TestWrapper(10);
        let b = TestWrapper(10);
        let c = TestWrapper(20);

        assert_eq!(a, b);
        assert_ne!(a, c);

        // Test against inner type
        assert_eq!(a, 10u64);
        assert_ne!(a, 20u64);
    }

    #[test]
    fn test_wrapper_partial_ord() {
        let a = TestWrapper(10);
        let b = TestWrapper(20);

        assert!(a < b);
        assert!(b > a);
        assert!(a <= a);
        assert!(a >= a);

        // Test against inner type
        assert!(a < 20u64);
        assert!(b > 10u64);
    }
}
