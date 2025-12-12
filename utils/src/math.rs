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

use core::cmp::Ordering;
use derive_more::Deref;
use ruint;
use sails_rs::{
    scale_info::{build::Fields, Path, Type},
    ActorId, Decode, Encode, TypeInfo, H160, H256,
};

pub use sails_rs::U256;

// ==============================================================================
//                              TRAITS
// ==============================================================================

pub trait Math:
    Max + Min + One + Zero + CheckedMath + PartialEq + From<NonZero<Self>> + TryInto<NonZero<Self>>
{
}

impl<
        T: Max
            + Min
            + One
            + Zero
            + CheckedMath
            + PartialEq
            + From<NonZero<Self>>
            + TryInto<NonZero<Self>>,
    > Math for T
{
}

pub trait CheckedMath: Sized {
    fn checked_add(self, rhs: Self) -> Option<Self>;
    fn checked_sub(self, rhs: Self) -> Option<Self>;

    fn checked_add_err(self, rhs: Self) -> Result<Self, OverflowError> {
        self.checked_add(rhs).ok_or(OverflowError)
    }

    fn checked_sub_err(self, rhs: Self) -> Result<Self, UnderflowError> {
        self.checked_sub(rhs).ok_or(UnderflowError)
    }
}

pub trait Max {
    const MAX: Self;
    fn is_max(&self) -> bool
    where
        Self: Sized + PartialEq,
    {
        *self == Self::MAX
    }
}

pub trait Min {
    const MIN: Self;
    fn is_min(&self) -> bool
    where
        Self: Sized + PartialEq,
    {
        *self == Self::MIN
    }
}

pub trait One {
    const ONE: Self;
    fn is_one(&self) -> bool
    where
        Self: Sized + PartialEq,
    {
        *self == Self::ONE
    }
}

pub trait Zero {
    const ZERO: Self;
    fn is_zero(&self) -> bool
    where
        Self: Sized + PartialEq,
    {
        *self == Self::ZERO
    }
}

// ==============================================================================
//                              MACROS
// ==============================================================================

/// Unifies implementation of all math traits for primitives to avoid repetition.
macro_rules! impl_primitive_traits {
    ($($t:ty),*) => {
        $(
            impl CheckedMath for $t {
                #[inline] fn checked_add(self, rhs: Self) -> Option<Self> { self.checked_add(rhs) }
                #[inline] fn checked_sub(self, rhs: Self) -> Option<Self> { self.checked_sub(rhs) }
            }
            impl Max for $t { const MAX: Self = <$t>::MAX; }
            impl Min for $t { const MIN: Self = <$t>::MIN; }
            impl One for $t { const ONE: Self = 1; }
            impl Zero for $t { const ZERO: Self = 0; }
        )*
    };
}

/// Helper for implementing conversions for NonZero wrapper.
macro_rules! impl_non_zero_conversion {
    ($($name: ident),*) => {
        $(
            impl TryFrom<$name> for NonZero<$name> {
                type Error = ZeroError;
                fn try_from(value: $name) -> Result<Self, Self::Error> { NonZero::try_new(value) }
            }
            impl From<NonZero<$name>> for $name {
                fn from(value: NonZero<$name>) -> Self { value.into_inner() }
            }
        )*
    };
}

/// Macro to implement Math traits and conversions for a wrapper type.
///
/// # Usage
///
/// ## 1. Default Mode
/// ```rust,ignore
/// impl_math_wrapper!(MyType, InnerType);
/// ```
/// Use this mode when you want standard, safe `TryFrom` implementations.
/// It automatically generates:
/// - `impl TryFrom<MyType> for u128`
/// - `impl TryFrom<MyType> for U256`
///
/// ## 2. Manual From Mode
/// ```rust,ignore
/// impl_math_wrapper!(MyType, InnerType, manual_from);
/// ```
/// Use this mode if you intend to implement `From<MyType> for u128` (or U256) manually.
///
/// Since implementing `From` automatically provides a blanket `TryFrom` implementation in Rust core,
/// the macro must skip generating `TryFrom` to avoid conflicting implementation errors (E0119).
#[macro_export]
macro_rules! impl_math_wrapper {
    // 1. Default Mode: Generates safe `TryFrom` conversions.
    ($wrapper:ident, $inner:ty) => {
        $crate::impl_math_wrapper!(@common $wrapper, $inner);

        // Standard TryFrom implementations.
        // We use TryFrom because converting a wrapper to u128/U256 might fail (overflow),
        // or the inner type's conversion is fallible.

        impl TryFrom<$wrapper> for u128 {
             type Error = $crate::math::OverflowError;
             fn try_from(value: $wrapper) -> Result<u128, Self::Error> {
                 value.0.try_into().map_err(|_| $crate::math::OverflowError)
             }
        }

        impl TryFrom<$wrapper> for $crate::math::U256 {
             type Error = $crate::math::OverflowError;
             fn try_from(value: $wrapper) -> Result<$crate::math::U256, Self::Error> {
                 value.0.try_into().map_err(|_| $crate::math::OverflowError)
             }
        }
    };

    // 2. Manual Mode: Skips output conversions.
    // Use this if you implement `From<$wrapper>` manually to avoid conflict.
    ($wrapper:ident, $inner:ty, manual_from) => {
        $crate::impl_math_wrapper!(@common $wrapper, $inner);
    };

    // 3. Common implementation details (Internal).
    (@common $wrapper:ident, $inner:ty) => {
        // --- CONSTANTS ---
        impl $crate::math::Max for $wrapper {
            const MAX: Self = Self(<$inner>::MAX);
        }
        impl $crate::math::Min for $wrapper {
            const MIN: Self = Self(<$inner>::MIN);
        }
        impl $crate::math::Zero for $wrapper {
            const ZERO: Self = Self(<$inner>::ZERO);
        }
        impl $crate::math::One for $wrapper {
            const ONE: Self = Self(<$inner>::ONE);
        }

        // --- MATH ---
        impl $crate::math::CheckedMath for $wrapper {
            fn checked_add(self, rhs: Self) -> Option<Self> {
                self.0.checked_add(rhs.0).map(Self)
            }
            fn checked_sub(self, rhs: Self) -> Option<Self> {
                self.0.checked_sub(rhs.0).map(Self)
            }
        }

        // --- NON ZERO ---
        impl TryFrom<$wrapper> for $crate::math::NonZero<$wrapper> {
            type Error = $crate::math::ZeroError;
            fn try_from(value: $wrapper) -> Result<Self, Self::Error> {
                $crate::math::NonZero::try_new(value)
            }
        }
        impl From<$crate::math::NonZero<$wrapper>> for $wrapper {
            fn from(value: $crate::math::NonZero<$wrapper>) -> Self {
                value.into_inner()
            }
        }

        // --- COMPARISONS (Wrapper vs NonZero) ---
        impl PartialEq<$crate::math::NonZero<$wrapper>> for $wrapper {
            fn eq(&self, other: &$crate::math::NonZero<$wrapper>) -> bool {
                self.eq(&other.0)
            }
        }
        impl PartialOrd<$crate::math::NonZero<$wrapper>> for $wrapper {
            fn partial_cmp(
                &self,
                other: &$crate::math::NonZero<$wrapper>,
            ) -> Option<core::cmp::Ordering> {
                self.partial_cmp(&other.0)
            }
        }

        // --- COMPARISONS (Wrapper vs Inner) ---
        impl PartialEq<$inner> for $wrapper {
            fn eq(&self, other: &$inner) -> bool {
                self.0 == *other
            }
        }

        impl PartialOrd<$inner> for $wrapper {
            fn partial_cmp(&self, other: &$inner) -> Option<core::cmp::Ordering> {
                self.0.partial_cmp(other)
            }
        }

        // --- INPUT CONVERSIONS (From External to Wrapper) ---

        // TryFrom U256 -> Wrapper
        impl TryFrom<$crate::math::U256> for $wrapper {
            type Error = $crate::math::OverflowError;
            fn try_from(value: $crate::math::U256) -> Result<Self, Self::Error> {
                let inner = <$inner>::try_from(value).map_err(|_| $crate::math::OverflowError)?;
                Ok(Self(inner))
            }
        }

        // TryFrom u128 -> Wrapper
        impl TryFrom<u128> for $wrapper {
            type Error = $crate::math::OverflowError;
            fn try_from(value: u128) -> Result<Self, Self::Error> {
                let inner = <$inner>::try_from(value).map_err(|_| $crate::math::OverflowError)?;
                Ok(Self(inner))
            }
        }
    };
}

// ==============================================================================
//                          PRIMITIVES & EXTERNAL TYPES
// ==============================================================================

impl_primitive_traits!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

// --- U256 ---
impl CheckedMath for U256 {
    fn checked_add(self, rhs: Self) -> Option<Self> {
        self.checked_add(rhs)
    }
    fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.checked_sub(rhs)
    }
}
impl Max for U256 {
    const MAX: Self = U256::MAX;
}
impl Min for U256 {
    const MIN: Self = U256::zero();
}
impl One for U256 {
    const ONE: Self = U256::one();
}
impl Zero for U256 {
    const ZERO: Self = U256::zero();
}

// --- ActorId, H256, H160 (No CheckedMath impl) ---

impl Max for ActorId {
    const MAX: Self = ActorId::new([u8::MAX; 32]);
}
impl Min for ActorId {
    const MIN: Self = ActorId::zero();
}
impl One for ActorId {
    const ONE: Self = {
        let mut b = [0; 32];
        b[12] = 1;
        Self::new(b)
    };
}
impl Zero for ActorId {
    const ZERO: Self = ActorId::zero();
}

impl Max for H256 {
    const MAX: Self = H256([u8::MAX; 32]);
}
impl Min for H256 {
    const MIN: Self = H256::zero();
}
impl One for H256 {
    const ONE: Self = {
        let mut b = [0; 32];
        b[0] = 1;
        H256(b)
    };
}
impl Zero for H256 {
    const ZERO: Self = H256::zero();
}

impl Max for H160 {
    const MAX: Self = H160([u8::MAX; 20]);
}
impl Min for H160 {
    const MIN: Self = H160::zero();
}
impl One for H160 {
    const ONE: Self = {
        let mut b = [0; 20];
        b[0] = 1;
        H160(b)
    };
}
impl Zero for H160 {
    const ZERO: Self = H160::zero();
}

// ==============================================================================
//                              CUSTOM UINT
// ==============================================================================

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CustomUint<const BITS: usize, const LIMBS: usize>(ruint::Uint<BITS, LIMBS>);

const fn n_bytes(bits: usize) -> usize {
    bits.div_ceil(8)
}

impl<const BITS: usize, const LIMBS: usize> CustomUint<BITS, LIMBS> {
    pub const fn new(val: ruint::Uint<BITS, LIMBS>) -> Self {
        Self(val)
    }

    pub fn into_inner(self) -> ruint::Uint<BITS, LIMBS> {
        self.0
    }

    /// Resizes the CustomUint to a different bit width.
    ///
    /// # Returns
    /// - `Ok(CustomUint<B2, L2>)`: A new `CustomUint` with the target size.
    /// - `Err(OverflowError)`: If the value is too large to fit in the target size.
    ///
    /// # Note
    /// use `try_into()` (via `TryFrom` trait) if you want to convert to external types
    /// like `u64`, `u128`, or `U256`.
    pub fn try_resize<const B2: usize, const L2: usize>(
        self,
    ) -> Result<CustomUint<B2, L2>, OverflowError> {
        let current_bytes = n_bytes(BITS);
        let target_bytes = n_bytes(B2);

        if target_bytes < current_bytes {
            // Check higher bytes for non-zero to detect overflow
            for i in target_bytes..current_bytes {
                if self.0.byte(i) != 0 {
                    return Err(OverflowError);
                }
            }
        }

        let mut buffer = sails_rs::vec![0u8; current_bytes];
        for (i, byte_val) in buffer.iter_mut().enumerate().take(current_bytes) {
            *byte_val = self.0.byte(i);
        }

        let copy_len = core::cmp::min(current_bytes, target_bytes);
        ruint::Uint::<B2, L2>::try_from_le_slice(&buffer[..copy_len])
            .ok_or(OverflowError)
            .map(CustomUint)
    }
}

// --- Traits for CustomUint ---

impl<const BITS: usize, const LIMBS: usize> Default for CustomUint<BITS, LIMBS> {
    fn default() -> Self {
        Self(ruint::Uint::ZERO)
    }
}

impl<const BITS: usize, const LIMBS: usize> core::fmt::Debug for CustomUint<BITS, LIMBS> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl<const BITS: usize, const LIMBS: usize> CheckedMath for CustomUint<BITS, LIMBS> {
    fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }
    fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }
}

impl<const BITS: usize, const LIMBS: usize> Max for CustomUint<BITS, LIMBS> {
    const MAX: Self = Self(ruint::Uint::MAX);
}
impl<const BITS: usize, const LIMBS: usize> Min for CustomUint<BITS, LIMBS> {
    const MIN: Self = Self(ruint::Uint::ZERO);
}
impl<const BITS: usize, const LIMBS: usize> One for CustomUint<BITS, LIMBS> {
    const ONE: Self = Self(ruint::Uint::ONE);
}
impl<const BITS: usize, const LIMBS: usize> Zero for CustomUint<BITS, LIMBS> {
    const ZERO: Self = Self(ruint::Uint::ZERO);
}

// --- Scale Codec Manual Implementation (Compact storage) ---

impl<const BITS: usize, const LIMBS: usize> Encode for CustomUint<BITS, LIMBS> {
    fn size_hint(&self) -> usize {
        n_bytes(BITS)
    }

    fn encode_to<T: sails_rs::scale_codec::Output + ?Sized>(&self, dest: &mut T) {
        let len = n_bytes(BITS);
        for i in 0..len {
            dest.push_byte(self.0.byte(i));
        }
    }
}

impl<const BITS: usize, const LIMBS: usize> Decode for CustomUint<BITS, LIMBS> {
    fn decode<I: sails_rs::scale_codec::Input>(
        input: &mut I,
    ) -> Result<Self, sails_rs::scale_codec::Error> {
        let len = n_bytes(BITS);
        let mut buffer = sails_rs::vec![0u8; len];
        input.read(&mut buffer[..len])?;
        ruint::Uint::try_from_le_slice(&buffer[..len])
            .ok_or("Overflow or invalid data".into())
            .map(CustomUint)
    }
}

impl<const BITS: usize, const LIMBS: usize> TypeInfo for CustomUint<BITS, LIMBS> {
    type Identity = Self;
    fn type_info() -> Type {
        Type::builder()
            .path(Path::new("CustomUint", module_path!()))
            .type_params(sails_rs::Vec::new())
            .composite(Fields::unnamed().field(|f| f.ty::<[u8]>()))
    }
}

// --- Converisons ---

impl<const BITS: usize, const LIMBS: usize> From<u64> for CustomUint<BITS, LIMBS> {
    fn from(val: u64) -> Self {
        Self(ruint::Uint::from(val))
    }
}

impl<const BITS: usize, const LIMBS: usize> TryFrom<u128> for CustomUint<BITS, LIMBS> {
    type Error = OverflowError;
    fn try_from(val: u128) -> Result<Self, Self::Error> {
        ruint::Uint::try_from(val)
            .map(Self)
            .map_err(|_| OverflowError)
    }
}

impl<const BITS: usize, const LIMBS: usize> TryFrom<U256> for CustomUint<BITS, LIMBS> {
    type Error = OverflowError;
    fn try_from(value: U256) -> Result<Self, Self::Error> {
        let mut bytes = [0u8; 32];
        value.to_little_endian(&mut bytes);
        // Find highest byte
        let len = bytes
            .iter()
            .rposition(|&x| x != 0)
            .map(|i| i + 1)
            .unwrap_or(0);
        ruint::Uint::try_from_le_slice(&bytes[..len])
            .ok_or(OverflowError)
            .map(Self)
    }
}

impl<const BITS: usize, const LIMBS: usize> TryFrom<CustomUint<BITS, LIMBS>> for u64 {
    type Error = OverflowError;
    fn try_from(value: CustomUint<BITS, LIMBS>) -> Result<Self, Self::Error> {
        value.0.try_into().map_err(|_| OverflowError)
    }
}

impl<const BITS: usize, const LIMBS: usize> TryFrom<CustomUint<BITS, LIMBS>> for u128 {
    type Error = OverflowError;
    fn try_from(value: CustomUint<BITS, LIMBS>) -> Result<Self, Self::Error> {
        value.0.try_into().map_err(|_| OverflowError)
    }
}

// Used when converting CustomUint to external type U256
impl<const BITS: usize, const LIMBS: usize> TryFrom<CustomUint<BITS, LIMBS>> for U256 {
    type Error = OverflowError;
    fn try_from(value: CustomUint<BITS, LIMBS>) -> Result<Self, Self::Error> {
        let len = n_bytes(BITS);
        if len > 32 {
            for i in 32..len {
                if value.0.byte(i) != 0 {
                    return Err(OverflowError);
                }
            }
        }
        let mut bytes = [0u8; 32];
        for (i, byte_val) in bytes.iter_mut().enumerate().take(core::cmp::min(len, 32)) {
            *byte_val = value.0.byte(i);
        }
        Ok(U256::from_little_endian(&bytes))
    }
}

// ==============================================================================
//                              NON ZERO
// ==============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Decode, Encode, TypeInfo, Hash, Deref)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct NonZero<T>(T);

impl<T: Zero + PartialEq> NonZero<T> {
    pub fn try_new(value: T) -> Result<Self, ZeroError> {
        (!value.is_zero()).then_some(Self(value)).ok_or(ZeroError)
    }

    pub fn try_add(self, rhs: Self) -> Result<Self, MathError>
    where
        T: CheckedMath,
    {
        let res = self.0.checked_add(rhs.0).ok_or(OverflowError)?;
        Self::try_new(res).map_err(Into::into)
    }

    pub fn try_sub(self, rhs: Self) -> Result<Self, MathError>
    where
        T: CheckedMath,
    {
        let result = self.0.checked_sub(rhs.0).ok_or(UnderflowError)?;
        Self::try_new(result).map_err(Into::into)
    }
}

impl<T> NonZero<T> {
    pub fn into_inner(self) -> T {
        self.0
    }

    /// Casts the underlying value `T` to `U` using `From`.
    ///
    /// This consumes the `NonZero` wrapper and returns the converted value `U`.
    /// Equivalent to `U::from(self.into_inner())`.
    pub fn cast<U: From<T>>(self) -> U {
        U::from(self.0)
    }

    /// Tries to cast the underlying value `T` to `U` using `TryFrom`.
    ///
    /// This consumes the `NonZero` wrapper and attempts to convert the inner value.
    /// Equivalent to `U::try_from(self.into_inner())`.
    pub fn try_cast<U: TryFrom<T>>(self) -> Result<U, U::Error> {
        U::try_from(self.0)
    }

    /// Cast NonZero<T> to NonZero<U> via `From`.
    pub fn non_zero_cast<U: From<T>>(self) -> NonZero<U> {
        NonZero(U::from(self.0))
    }
}

impl<T: PartialEq> PartialEq<T> for NonZero<T> {
    fn eq(&self, other: &T) -> bool {
        self.0.eq(other)
    }
}

// Manual Ord implementation to avoid recursion issues and be explicit
impl<T: Ord> Ord for NonZero<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: PartialOrd> PartialOrd for NonZero<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<T: PartialOrd> PartialOrd<T> for NonZero<T> {
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

// Implement conversions
impl_non_zero_conversion!(ActorId, H256, H160, U256);
impl_non_zero_conversion!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

impl<const BITS: usize, const LIMBS: usize> TryFrom<CustomUint<BITS, LIMBS>>
    for NonZero<CustomUint<BITS, LIMBS>>
{
    type Error = ZeroError;
    fn try_from(value: CustomUint<BITS, LIMBS>) -> Result<Self, Self::Error> {
        NonZero::try_new(value)
    }
}

impl<const BITS: usize, const LIMBS: usize> From<NonZero<CustomUint<BITS, LIMBS>>>
    for CustomUint<BITS, LIMBS>
{
    fn from(value: NonZero<CustomUint<BITS, LIMBS>>) -> Self {
        value.into_inner()
    }
}

// ==============================================================================
//                              ERRORS
// ==============================================================================

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo, thiserror::Error,
)]
#[codec(crate = sails_rs::scale_codec)]
#[error(transparent)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum MathError {
    Overflow(#[from] OverflowError),
    Underflow(#[from] UnderflowError),
    Zero(#[from] ZeroError),
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Decode,
    Encode,
    TypeInfo,
    thiserror::Error,
)]
#[codec(crate = sails_rs::scale_codec)]
#[error("mathematical overflow")]
#[scale_info(crate = sails_rs::scale_info)]
pub struct OverflowError;

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Decode,
    Encode,
    TypeInfo,
    thiserror::Error,
)]
#[codec(crate = sails_rs::scale_codec)]
#[error("mathematical underflow")]
#[scale_info(crate = sails_rs::scale_info)]
pub struct UnderflowError;

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Decode,
    Encode,
    TypeInfo,
    thiserror::Error,
)]
#[codec(crate = sails_rs::scale_codec)]
#[error("zero error")]
#[scale_info(crate = sails_rs::scale_info)]
pub struct ZeroError;
