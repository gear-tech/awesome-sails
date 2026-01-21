#![no_std]

#[cfg(feature = "vft-service")]
pub use awesome_sails_vft_service as vft;

#[cfg(feature = "vft-service-utils")]
pub use awesome_sails_vft_service_utils as vft_utils;

#[cfg(feature = "vft-admin-service")]
pub use awesome_sails_vft_admin_service as vft_admin;

#[cfg(feature = "vft-extension-service")]
pub use awesome_sails_vft_extension_service as vft_extension;

#[cfg(feature = "vft-metadata-service")]
pub use awesome_sails_vft_metadata_service as vft_metadata;

#[cfg(feature = "vft-native-exchange-service")]
pub use awesome_sails_vft_native_exchange_service as vft_native_exchange;

#[cfg(feature = "vft-native-exchange-admin-service")]
pub use awesome_sails_vft_native_exchange_admin_service as vft_native_exchange_admin;

#[cfg(feature = "access-control")]
pub use awesome_sails_access_control_service as access_control;
