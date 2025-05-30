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

//! Awesome math module.

use crate::{ensure, impl_non_zero_conversion};
use core::cmp::Ordering;
use sails_rs::{ActorId, Decode, Encode, H160, H256, TypeInfo};

pub use sails_rs::U256;

macro_rules! for_primitives {
    ($macro:ident) => {
        $macro!(
            u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
        );
    };
}

/// Aggregation trait for math operations from the module.
pub trait Math:
    Max + Min + One + Zero + CheckedMath + PartialEq + From<NonZero<Self>> + TryInto<NonZero<Self>>
{
}

impl<
    T: Max + Min + One + Zero + CheckedMath + PartialEq + From<NonZero<Self>> + TryInto<NonZero<Self>>,
> Math for T
{
}

/// A trait that defines checked math operations for a type.
pub trait CheckedMath: Sized {
    /// Performs checked addition.
    fn checked_add(self, rhs: Self) -> Option<Self>;

    /// Performs checked addition and returns an error if overflow occurs.
    fn checked_add_err(self, rhs: Self) -> Result<Self, OverflowError> {
        self.checked_add(rhs).ok_or(OverflowError)
    }

    /// Performs checked subtraction.
    fn checked_sub(self, rhs: Self) -> Option<Self>;

    /// Performs checked subtraction and returns an error if underflow occurs.
    fn checked_sub_err(self, rhs: Self) -> Result<Self, UnderflowError> {
        self.checked_sub(rhs).ok_or(UnderflowError)
    }
}

macro_rules! impl_checked_math {
    ($($t:ty),*) => {
        $(
            impl CheckedMath for $t {
                fn checked_add(self, rhs: Self) -> Option<Self> {
                    self.checked_add(rhs)
                }

                fn checked_sub(self, rhs: Self) -> Option<Self> {
                    self.checked_sub(rhs)
                }
            }
        )*
    };
}

for_primitives!(impl_checked_math);
impl_checked_math!(U256);

/// A trait that defines a maximum value for a type.
pub trait Max {
    /// The maximum value of the type.
    const MAX: Self;

    /// Checks if the current value is maximum.
    fn is_max(&self) -> bool
    where
        Self: Sized + PartialEq,
    {
        *self == Self::MAX
    }
}

macro_rules! impl_max {
    ($($t:ty),*) => {
        $(
            impl Max for $t {
                const MAX: Self = <$t>::MAX;
            }
        )*
    };
}

for_primitives!(impl_max);
impl_max!(U256);

impl Max for ActorId {
    const MAX: Self = ActorId::new([u8::MAX; 32]);
}

impl Max for H256 {
    const MAX: Self = H256([u8::MAX; 32]);
}

impl Max for H160 {
    const MAX: Self = H160([u8::MAX; 20]);
}

/// A trait that defines a minimum value for a type.
pub trait Min {
    /// The minimum value of the type.
    const MIN: Self;

    /// Checks if the current value is minimum.
    fn is_min(&self) -> bool
    where
        Self: Sized + PartialEq,
    {
        *self == Self::MIN
    }
}

macro_rules! impl_min {
    ($($t:ty),*) => {
        $(
            impl Min for $t {
                const MIN: Self = <$t>::MIN;
            }
        )*
    };
}

for_primitives!(impl_min);

impl Min for U256 {
    const MIN: Self = U256::zero();
}

impl Min for ActorId {
    const MIN: Self = ActorId::zero();
}

impl Min for H256 {
    const MIN: Self = H256([0; 32]);
}

impl Min for H160 {
    const MIN: Self = H160([0; 20]);
}

/// A trait that defines a **one** value for a type.
pub trait One {
    /// The one value of the type.
    const ONE: Self;

    /// Checks if the current value is one.
    fn is_one(&self) -> bool
    where
        Self: Sized + PartialEq,
    {
        *self == Self::ONE
    }
}

macro_rules! impl_one {
    ($($t:ty),*) => {
        $(
            impl One for $t {
                const ONE: Self = 1;
            }
        )*
    };
}

for_primitives!(impl_one);

impl One for U256 {
    const ONE: Self = U256::one();
}

impl One for ActorId {
    const ONE: Self = {
        let mut bytes = [0; 32];
        bytes[12] = 1;
        Self::new(bytes)
    };
}

impl One for H256 {
    const ONE: Self = {
        let mut bytes = [0; 32];
        bytes[0] = 1;
        H256(bytes)
    };
}

impl One for H160 {
    const ONE: Self = {
        let mut bytes = [0; 20];
        bytes[0] = 1;
        H160(bytes)
    };
}

/// A trait that defines a **zero** value for a type.
pub trait Zero {
    /// The zero value of the type.
    const ZERO: Self;

    /// Checks if the current value is zero.
    fn is_zero(&self) -> bool
    where
        Self: Sized + PartialEq,
    {
        *self == Self::ZERO
    }
}

macro_rules! impl_zero {
    ($($t:ty),*) => {
        $(
            impl Zero for $t {
                const ZERO: Self = 0;
            }
        )*
    };
}

for_primitives!(impl_zero);

impl Zero for U256 {
    const ZERO: Self = U256::zero();
}

impl Zero for ActorId {
    const ZERO: Self = ActorId::zero();
}

impl Zero for H256 {
    const ZERO: Self = H256::zero();
}

impl Zero for H160 {
    const ZERO: Self = H160::zero();
}

/// This struct is a wrapper around an array of `N` bytes, where the bytes are
/// stored in little-endian order.
///
/// It is useful for representing fixed-size numeric values in a compact manner.
///
/// NOTE: there's useful impl_math_for_small_le_bytes_wrap! macro for deriving wrappers.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Decode,
    Encode,
    TypeInfo,
    Hash,
    derive_more::From,
    derive_more::Into,
    derive_more::AsRef,
)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct LeBytes<const N: usize>([u8; N]);

impl<const N: usize> LeBytes<N> {
    /// Creates a new `LeBytes` instance from a byte array.
    pub const fn new(bytes: [u8; N]) -> Self {
        Self(bytes)
    }

    /// Tries to convert a given `LeBytes` instance to another `LeBytes` instance.
    pub fn try_convert_from<const M: usize>(value: LeBytes<M>) -> Result<Self, OverflowError> {
        let mut me = [0; N];

        if M <= N {
            me[..M].copy_from_slice(&value.0);
            return Ok(Self(me));
        }

        let (to_copy, to_strip) = value.0.split_at(N);

        ensure!(to_strip.iter().all(|&b| b == 0), OverflowError);

        me.copy_from_slice(to_copy);

        Ok(Self(me))
    }

    /// Returns the inner byte array.
    pub fn as_bytes(&self) -> &[u8; N] {
        &self.0
    }

    /// Tries to convert self into another `LeBytes` instance.
    pub fn try_convert_into<const M: usize>(self) -> Result<LeBytes<M>, OverflowError> {
        LeBytes::<M>::try_convert_from(self)
    }
}

impl<const N: usize> Default for LeBytes<N> {
    fn default() -> Self {
        Self([0; N])
    }
}

impl<const N: usize> PartialOrd for LeBytes<N> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<const N: usize> Ord for LeBytes<N> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.iter().rev().cmp(other.0.iter().rev())
    }
}

impl<const N: usize> TryFrom<u128> for LeBytes<N> {
    type Error = OverflowError;

    fn try_from(value: u128) -> Result<Self, Self::Error> {
        Self::try_convert_from(LeBytes(value.to_le_bytes()))
    }
}

impl<const N: usize> TryInto<u128> for LeBytes<N> {
    type Error = OverflowError;

    fn try_into(self) -> Result<u128, Self::Error> {
        LeBytes::<16>::try_convert_from(self).map(|LeBytes(b)| u128::from_le_bytes(b))
    }
}

impl<const N: usize> TryFrom<U256> for LeBytes<N> {
    type Error = OverflowError;

    fn try_from(value: U256) -> Result<Self, Self::Error> {
        let mut bytes = [0; 32];
        value.to_little_endian(&mut bytes);

        Self::try_convert_from(LeBytes(bytes))
    }
}

impl<const N: usize> TryInto<U256> for LeBytes<N> {
    type Error = OverflowError;

    fn try_into(self) -> Result<U256, Self::Error> {
        LeBytes::<32>::try_convert_from(self).map(|LeBytes(b)| U256::from_little_endian(&b))
    }
}

impl<const N: usize> CheckedMath for LeBytes<N> {
    fn checked_add(self, rhs: Self) -> Option<Self> {
        let mut result = [0; N];
        let mut carry = 0u16;

        for (i, res_i) in result.iter_mut().enumerate() {
            let sum = self.0[i] as u16 + rhs.0[i] as u16 + carry;
            *res_i = sum as u8;
            carry = sum >> 8;
        }

        (carry == 0).then_some(Self(result))
    }

    fn checked_sub(self, rhs: Self) -> Option<Self> {
        let mut result = [0u8; N];
        let mut borrowed = false;

        for (i, res_i) in result.iter_mut().enumerate() {
            let diff = self.0[i] as i16 - rhs.0[i] as i16 - borrowed as i16;

            if diff < 0 {
                *res_i = (diff + 256) as u8;
                borrowed = true;
            } else {
                *res_i = diff as u8;
                borrowed = false;
            }
        }

        (!borrowed).then_some(Self(result))
    }
}

impl<const N: usize> Max for LeBytes<N> {
    const MAX: Self = LeBytes([u8::MAX; N]);
}

impl<const N: usize> Min for LeBytes<N> {
    const MIN: Self = LeBytes([0; N]);
}

impl<const N: usize> One for LeBytes<N> {
    const ONE: Self = {
        let mut bytes = [0; N];
        bytes[N - 1] = 1;
        Self(bytes)
    };
}

impl<const N: usize> Zero for LeBytes<N> {
    const ZERO: Self = Self([0; N]);
}

/// A wrapper around a type `T` that guarantees it is non-zero.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Decode,
    Encode,
    TypeInfo,
    Hash,
    derive_more::Deref,
)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct NonZero<T>(T);

impl<T: Zero + PartialEq> NonZero<T> {
    /// Creates a new [`NonZero`] instance if the value is non-zero.
    pub fn try_new(value: T) -> Result<Self, ZeroError> {
        (!value.is_zero()).then_some(Self(value)).ok_or(ZeroError)
    }

    /// Tries to sum up two [`NonZero`] values.
    pub fn try_add(self, rhs: Self) -> Result<Self, MathError>
    where
        T: CheckedMath,
    {
        let res = self.0.checked_add(rhs.0).ok_or(OverflowError)?;
        Self::try_new(res).map_err(Into::into)
    }

    /// Tries to subtract two [`NonZero`] values.
    pub fn try_sub(self, rhs: Self) -> Result<Self, MathError>
    where
        T: CheckedMath,
    {
        let result = self.0.checked_sub(rhs.0).ok_or(UnderflowError)?;
        Self::try_new(result).map_err(Into::into)
    }
}

impl<T> NonZero<T> {
    /// Converts the [`NonZero`] instance back into its inner value.
    pub fn into_inner(self) -> T {
        self.0
    }

    /// Converts the [`NonZero`] instance into another convertible from inner type.
    pub fn cast<U: From<T>>(self) -> U {
        U::from(self.0)
    }

    /// Tries to converts the [`NonZero`] instance into another convertible from inner type.
    pub fn try_cast<U: TryFrom<T>>(self) -> Result<U, U::Error> {
        U::try_from(self.0)
    }

    /// Converts the [`NonZero`] instance into a new [`NonZero`] instance of a different type.
    pub fn non_zero_cast<U: From<T>>(self) -> NonZero<U> {
        NonZero(U::from(self.0))
    }
}

impl<T: PartialEq> PartialEq<T> for NonZero<T> {
    fn eq(&self, other: &T) -> bool {
        self.0.eq(other)
    }
}

impl<T: PartialOrd> PartialOrd<T> for NonZero<T> {
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

for_primitives!(impl_non_zero_conversion);
impl_non_zero_conversion!(ActorId, H256, H160, U256);

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

/// Arbitrary error type for math errors.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo, thiserror::Error,
)]
#[codec(crate = sails_rs::scale_codec)]
#[error(transparent)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum MathError {
    /// Overflow error.
    Overflow(#[from] OverflowError),
    /// Underflow error.
    Underflow(#[from] UnderflowError),
    /// Zero error.
    Zero(#[from] ZeroError),
}

/// Arbitrary error type for math overflows.
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

/// Arbitrary error type for math underflows.
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

/// Arbitrary error type for zeroes.
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
