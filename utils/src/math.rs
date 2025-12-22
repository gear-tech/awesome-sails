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

use alloc::vec;
use bnum::BUintD8;
use core::cmp::Ordering;
use derive_more::Deref;
use parity_scale_codec::{Decode, Encode};
use scale_info::{Path, Type, TypeInfo, build::Fields};

pub use primitive_types::{H160, H256, U256};

#[cfg(feature = "gprimitives")]
pub use gprimitives::ActorId;

// ==============================================================================
//                              TRAITS
// ==============================================================================

pub trait Math:
    Max + Min + One + Zero + CheckedMath + PartialEq + From<NonZero<Self>> + TryInto<NonZero<Self>>
{
}

impl<
    T: Max + Min + One + Zero + CheckedMath + PartialEq + From<NonZero<Self>> + TryInto<NonZero<Self>>,
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

// ==============================================================================
//                          PRIMITIVES & EXTERNAL TYPES
// ==============================================================================

impl_primitive_traits!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

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

#[cfg(feature = "gprimitives")]
impl Max for ActorId {
    const MAX: Self = ActorId::new([u8::MAX; 32]);
}

#[cfg(feature = "gprimitives")]
impl Min for ActorId {
    const MIN: Self = ActorId::zero();
}

#[cfg(feature = "gprimitives")]
impl One for ActorId {
    const ONE: Self = {
        let mut b = [0; 32];
        b[12] = 1;
        Self::new(b)
    };
}

#[cfg(feature = "gprimitives")]
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
//                              CUSTOM UINT (with bnum::BUintD8)
// ==============================================================================

/// A custom unsigned integer wrapper using `bnum::BUintD8`.
///
/// Unlike standard `ruint` or `bnum::BUint` which align to u64 (8 bytes),
/// `BUintD8` aligns to u8 (1 byte). This allows creating integers of arbitrary
/// byte size (e.g., 10 bytes for 80 bits) without memory overhead.
///
/// `N` is the number of **bytes** (not bits).
/// Example: For 80 bits, use `LeBytes<10>`.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::From,
    derive_more::Into,
    derive_more::AsRef,
)]
pub struct LeBytes<const N: usize>(BUintD8<N>);

impl<const N: usize> LeBytes<N> {
    pub const fn new(val: BUintD8<N>) -> Self {
        Self(val)
    }

    pub fn into_inner(self) -> BUintD8<N> {
        self.0
    }

    /// Resizes the LeBytes to a different **byte** width.
    ///
    /// # Returns
    /// - `Ok(LeBytes<N2>)`: A new `LeBytes` with the target size.
    /// - `Err(OverflowError)`: If the value is too large to fit in the target size.
    pub fn try_resize<const N2: usize>(self) -> Result<LeBytes<N2>, OverflowError> {
        let digits = self.0.digits();

        if N2 >= N {
            // Expanding: copy existing bytes to new larger array
            let mut new_digits = [0u8; N2];
            new_digits[..N].copy_from_slice(digits);
            // Safe to unwrap here because N2 >= N
            Ok(LeBytes(BUintD8::from_digits(new_digits)))
        } else {
            // Shrinking: check for overflow
            // Check if any byte from N2 to N is non-zero
            if digits[N2..].iter().any(|&x| x != 0) {
                return Err(OverflowError);
            }

            let mut new_digits = [0u8; N2];
            new_digits.copy_from_slice(&digits[..N2]);
            Ok(LeBytes(BUintD8::from_digits(new_digits)))
        }
    }
}

// --- Traits for LeBytes ---

impl<const N: usize> Default for LeBytes<N> {
    fn default() -> Self {
        Self(BUintD8::ZERO)
    }
}

impl<const N: usize> core::fmt::Debug for LeBytes<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl<const N: usize> CheckedMath for LeBytes<N> {
    fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }
    fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }
}

impl<const N: usize> Max for LeBytes<N> {
    const MAX: Self = Self(BUintD8::MAX);
}
impl<const N: usize> Min for LeBytes<N> {
    const MIN: Self = Self(BUintD8::MIN);
}
impl<const N: usize> One for LeBytes<N> {
    const ONE: Self = Self(BUintD8::ONE);
}
impl<const N: usize> Zero for LeBytes<N> {
    const ZERO: Self = Self(BUintD8::ZERO);
}

// --- Scale Codec Manual Implementation (Compact storage) ---

impl<const N: usize> Encode for LeBytes<N> {
    fn size_hint(&self) -> usize {
        N
    }

    fn encode_to<T: parity_scale_codec::Output + ?Sized>(&self, dest: &mut T) {
        // BUintD8 stores digits as Little Endian bytes directly
        for byte in self.0.digits() {
            dest.push_byte(*byte);
        }
    }
}

impl<const N: usize> Decode for LeBytes<N> {
    fn decode<I: parity_scale_codec::Input>(
        input: &mut I,
    ) -> Result<Self, parity_scale_codec::Error> {
        let mut buffer = vec![0u8; N];
        input.read(&mut buffer[..N])?;

        let mut bytes = [0u8; N];
        bytes.copy_from_slice(&buffer);

        // BUintD8 uses from_digits for constructor
        Ok(LeBytes(BUintD8::from_digits(bytes)))
    }
}

impl<const N: usize> TypeInfo for LeBytes<N> {
    type Identity = Self;
    fn type_info() -> Type {
        Type::builder()
            .path(Path::new("LeBytes", module_path!()))
            .type_params(vec::Vec::new())
            .composite(Fields::unnamed().field(|f| f.ty::<[u8]>()))
    }
}

// --- Converisons ---

impl<const N: usize> TryFrom<u128> for LeBytes<N> {
    type Error = OverflowError;
    fn try_from(val: u128) -> Result<Self, Self::Error> {
        // MANUAL SAFE IMPLEMENTATION
        // We do this manually because bnum::BUintD8::try_from(u128) panics
        // if the target type is smaller than u128 (e.g. 8 bytes).
        let bytes = val.to_le_bytes(); // [u8; 16]

        if N >= 16 {
            // Target is large enough, just copy.
            let mut my_bytes = [0u8; N];
            my_bytes[..16].copy_from_slice(&bytes);
            Ok(Self(BUintD8::from_digits(my_bytes)))
        } else {
            // Target is smaller (e.g. N=10 or N=8).
            // Check if any byte beyond N is non-zero.
            if bytes[N..].iter().any(|&b| b != 0) {
                return Err(OverflowError);
            }

            // Safe to copy lower N bytes
            let mut my_bytes = [0u8; N];
            my_bytes.copy_from_slice(&bytes[..N]);
            Ok(Self(BUintD8::from_digits(my_bytes)))
        }
    }
}

impl<const N: usize> TryFrom<U256> for LeBytes<N> {
    type Error = OverflowError;
    fn try_from(value: U256) -> Result<Self, Self::Error> {
        let mut bytes = [0u8; 32];
        value.to_little_endian(&mut bytes);

        // Check for overflow if N < 32
        if N < 32 {
            for &byte in bytes.iter().skip(N) {
                if byte != 0 {
                    return Err(OverflowError);
                }
            }
        }

        let mut my_bytes = [0u8; N];
        let copy_len = core::cmp::min(N, 32);
        my_bytes[..copy_len].copy_from_slice(&bytes[..copy_len]);

        Ok(Self(BUintD8::from_digits(my_bytes)))
    }
}

impl<const N: usize> TryFrom<LeBytes<N>> for u64 {
    type Error = OverflowError;
    fn try_from(value: LeBytes<N>) -> Result<Self, Self::Error> {
        value.0.try_into().map_err(|_| OverflowError)
    }
}

impl<const N: usize> TryFrom<LeBytes<N>> for u128 {
    type Error = OverflowError;
    fn try_from(value: LeBytes<N>) -> Result<Self, Self::Error> {
        value.0.try_into().map_err(|_| OverflowError)
    }
}

// Used when converting LeBytes to external type U256
impl<const N: usize> TryFrom<LeBytes<N>> for U256 {
    type Error = OverflowError;
    fn try_from(value: LeBytes<N>) -> Result<Self, Self::Error> {
        let digits = value.0.digits();
        let len = N;

        // If our custom int is larger than 256 bits (32 bytes), check for overflow
        if len > 32 {
            for &digit in digits.iter().take(len).skip(32) {
                if digit != 0 {
                    return Err(OverflowError);
                }
            }
        }

        let mut bytes = [0u8; 32];
        let copy_len = core::cmp::min(len, 32);
        bytes[..copy_len].copy_from_slice(&digits[..copy_len]);

        Ok(U256::from_little_endian(&bytes))
    }
}

// ==============================================================================
//                              NON ZERO
// ==============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Decode, Encode, TypeInfo, Hash, Deref)]
#[codec(crate = parity_scale_codec)]
#[scale_info(crate = scale_info)]
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
#[cfg(feature = "gprimitives")]
impl_non_zero_conversion!(ActorId);

impl_non_zero_conversion!(H256, H160, U256);
impl_non_zero_conversion!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

impl<const N: usize> TryFrom<LeBytes<N>> for NonZero<LeBytes<N>> {
    type Error = ZeroError;
    fn try_from(value: LeBytes<N>) -> Result<Self, Self::Error> {
        NonZero::try_new(value)
    }
}

impl<const N: usize> From<NonZero<LeBytes<N>>> for LeBytes<N> {
    fn from(value: NonZero<LeBytes<N>>) -> Self {
        value.into_inner()
    }
}

// ==============================================================================
//                              ERRORS
// ==============================================================================

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo, thiserror::Error,
)]
#[codec(crate = parity_scale_codec)]
#[error(transparent)]
#[scale_info(crate = scale_info)]
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
#[codec(crate = parity_scale_codec)]
#[error("mathematical overflow")]
#[scale_info(crate = scale_info)]
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
#[codec(crate = parity_scale_codec)]
#[error("mathematical underflow")]
#[scale_info(crate = scale_info)]
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
#[codec(crate = parity_scale_codec)]
#[error("zero error")]
#[scale_info(crate = scale_info)]
pub struct ZeroError;
