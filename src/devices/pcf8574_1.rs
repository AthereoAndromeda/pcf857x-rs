use core::cell;
#[cfg(not(feature = "async"))]
use embedded_hal::i2c::I2c;
#[cfg(feature = "async")]
use embedded_hal_async::i2c::I2c;

use crate::split_pins::pcf8574;
use crate::{Error, PinFlag, SlaveAddr};

macro_rules! pcf8574 {
    ( $device_name:ident, $device_data_name:ident, $default_address:expr ) => {
        /// Device driver
        #[derive(Debug, Default)]
        pub struct $device_name<I2C> {
            /// Data
            pub(crate) data: cell::RefCell<$device_data_name<I2C>>,
        }

        #[derive(Debug, Default)]
        pub(crate) struct $device_data_name<I2C> {
            /// The concrete I²C device implementation.
            pub(crate) i2c: I2C,
            /// The I²C device address.
            pub(crate) address: u8,
            /// Last status set to output pins, used to conserve its status while doing a read.
            pub(crate) last_set_mask: u8,
        }

        #[maybe_async_cfg::maybe(
            sync(cfg(not(feature = "async")), keep_self),
            async(feature = "async", keep_self)
        )]
        impl<I2C, E> $device_name<I2C>
        where
            I2C: I2c<Error = E>,
        {
            /// Create new instance of the device
            pub fn new(i2c: I2C, address: SlaveAddr) -> Self {
                let data = $device_data_name {
                    i2c,
                    address: address.addr($default_address),
                    last_set_mask: 0,
                };
                $device_name {
                    data: cell::RefCell::new(data),
                }
            }

            /// Destroy driver instance, return I²C bus instance.
            pub fn destroy(self) -> I2C {
                self.data.into_inner().i2c
            }

            cfg_if::cfg_if! {
                if #[cfg(feature = "async")] {
                    pub(crate) async fn do_on_acquired<R, Fut>(
                        &self,
                        f: impl FnOnce(cell::RefMut<$device_data_name<I2C>>) -> Fut,
                    ) -> Result<R, Error<E>>
                    where
                        Fut: core::future::Future<Output = Result<R, Error<E>>>, // Constraint for async
                    {
                        let dev = self
                            .data
                            .try_borrow_mut()
                            .map_err(|_| Error::CouldNotAcquireDevice)?;
                        f(dev).await
                    }
                } else {
                    pub(crate) fn do_on_acquired<R>(
                        &self,
                        f: impl FnOnce(cell::RefMut<$device_data_name<I2C>>) -> Result<R, Error<E>>,
                    ) -> Result<R, Error<E>> {
                        let dev = self
                            .data
                            .try_borrow_mut()
                            .map_err(|_| Error::CouldNotAcquireDevice)?;
                        f(dev)
                    }
                }
            }

            #[maybe_async_cfg::maybe(
                sync(cfg(not(feature = "async")), keep_self),
                async(feature = "async", keep_self)
            )]
            /// Set the status of all I/O pins.
            pub async fn set(&mut self, bits: u8) -> Result<(), Error<E>> {
                self.do_on_acquired(|dev| Self::_set(dev, bits)).await
                // cfg_if::cfg_if! {
                //     if #[cfg(feature = "async")] {
                //         self.do_on_acquired(async |dev| Self::_set(dev, bits).await).await
                //     } else {
                //         self.do_on_acquired(|dev| Self::_set(dev, bits))
                //     }
                // }
            }

            #[maybe_async_cfg::maybe(
                sync(cfg(not(feature = "async")), keep_self),
                async(feature = "async", keep_self)
            )]
            pub(crate) async fn _set(
                #[cfg(not(feature = "async"))] mut dev: cell::RefMut<'_, $device_data_name<I2C>>,
                #[cfg(feature = "async")] mut dev: cell::RefMut<'static, $device_data_name<I2C>>,
                bits: u8,
            ) -> Result<(), Error<E>> {
                let address = dev.address;
                dev.i2c.write(address, &[bits]).await.map_err(Error::I2C)?;
                dev.last_set_mask = bits;
                Ok(())
            }

            #[maybe_async_cfg::maybe(
                sync(cfg(not(feature = "async")), keep_self),
                async(feature = "async", keep_self)
            )]
            /// Set the status of all I/O pins repeatedly by looping through each array element
            pub async fn write_array(&mut self, data: &[u8]) -> Result<(), Error<E>> {
                if let Some(last) = data.last() {
                    cfg_if::cfg_if! {
                        if #[cfg(feature = "async")] {
                            self.do_on_acquired(|mut dev| async move {
                                let address = dev.address;
                                dev.i2c.write(address, &data).await.map_err(Error::I2C)?;
                                dev.last_set_mask = *last;
                                Ok(())
                            }).await?;
                        } else {
                            self.do_on_acquired(|mut dev| {
                                let address = dev.address;
                                dev.i2c.write(address, &data).map_err(Error::I2C)?;
                                dev.last_set_mask = *last;
                                Ok(())
                            })?;
                        }
                    }
                }
                Ok(())
            }

            /// Split device into individual pins
            pub fn split(&self) -> pcf8574::Parts<'_, $device_name<I2C>, E> {
                pcf8574::Parts::new(&self)
            }
        }

        #[maybe_async_cfg::maybe(
            sync(cfg(not(feature = "async")), keep_self),
            async(feature = "async", keep_self)
        )]
        impl<I2C, E> $device_name<I2C>
        where
            I2C: I2c<Error = E>,
        {
            #[maybe_async_cfg::maybe(
                sync(cfg(not(feature = "async")), keep_self),
                async(feature = "async", keep_self)
            )]
            /// Get the status of the selected I/O pins.
            /// The mask of the pins to be read can be created with a combination of
            /// `PinFlag::P0` to `PinFlag::P7`.
            pub async fn get(&mut self, mask: PinFlag) -> Result<u8, Error<E>> {
                if (mask.mask >> 8) != 0 {
                    return Err(Error::InvalidInputData);
                }

                cfg_if::cfg_if! {
                    if #[cfg(feature = "async")] {
                        self.do_on_acquired(async |dev| Self::_get(dev, mask).await)
                    } else {
                        self.do_on_acquired(|dev| Self::_get(dev, mask))
                    }
                }
            }

            #[maybe_async_cfg::maybe(
                sync(cfg(not(feature = "async")), keep_self),
                async(feature = "async", keep_self)
            )]
            pub(crate) async fn _get(
                mut dev: cell::RefMut<'_, $device_data_name<I2C>>,
                mask: PinFlag,
            ) -> Result<u8, Error<E>> {
                let mask = mask.mask as u8 | dev.last_set_mask;
                let address = dev.address;
                // configure selected pins as inputs
                dev.i2c.write(address, &[mask]).await.map_err(Error::I2C)?;

                let mut bits = [0];

                dev.i2c
                    .read(address, &mut bits)
                    .await
                    .map_err(Error::I2C)
                    .and(Ok(bits[0]))
            }

            /// Get the status of the selected I/O pins repeatedly and put them in the
            /// provided array.
            /// The mask of the pins to be read can be created with a combination of
            /// `PinFlag::P0` to `PinFlag::P7`.
            pub fn read_array(
                &mut self,
                mask: PinFlag,
                mut data: &mut [u8],
            ) -> Result<(), Error<E>> {
                if !data.is_empty() {
                    if (mask.mask >> 8) != 0 {
                        return Err(Error::InvalidInputData);
                    }
                    self.do_on_acquired(|mut dev| {
                        let mask = mask.mask as u8 | dev.last_set_mask;
                        let address = dev.address;
                        // configure selected pins as inputs
                        dev.i2c.write(address, &[mask]).map_err(Error::I2C)?;

                        dev.i2c.read(address, &mut data).map_err(Error::I2C)
                    })?;
                }
                Ok(())
            }
        }
    };
}

pcf8574!(Pcf8574, Pcf8574Data, 0b010_0000);
pcf8574!(Pcf8574a, Pcf8574aData, 0b011_1000);
