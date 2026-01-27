#![no_std]

#[cfg(feature = "vft")]
pub use awesome_sails_vft as vft;

#[cfg(feature = "vft-utils")]
pub use awesome_sails_vft_utils as vft_utils;

#[cfg(feature = "vft-admin")]
pub use awesome_sails_vft_admin as vft_admin;

#[cfg(feature = "vft-extension")]
pub use awesome_sails_vft_extension as vft_extension;

#[cfg(feature = "vft-metadata")]
pub use awesome_sails_vft_metadata as vft_metadata;

#[cfg(feature = "vft-native-exchange")]
pub use awesome_sails_vft_native_exchange as vft_native_exchange;

#[cfg(feature = "vft-native-exchange-admin")]
pub use awesome_sails_vft_native_exchange_admin as vft_native_exchange_admin;

#[cfg(feature = "access-control")]
pub use awesome_sails_access_control as access_control;
