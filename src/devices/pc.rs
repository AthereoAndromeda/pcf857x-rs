#![cfg(feature = "async")]
use core::cell::{RefCell, RefMut};

// use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
// use embassy_sync::mutex::Mutex;
use embedded_hal_async::i2c::I2c;

use crate::split_pins::pcf8574;
use crate::{Error, PinFlag, SlaveAddr};

/// Device driver
#[derive(Debug, Default)]
pub struct PcAsync<I2C> {
    /// Data
    pub(crate) data: RefCell<PcData<I2C>>,
}

#[derive(Debug, Default)]
pub(crate) struct PcData<I2C> {
    /// The concrete I²C device implementation.
    pub(crate) i2c: I2C,
    /// The I²C device address.
    pub(crate) address: u8,
    /// Last status set to output pins, used to conserve its status while doing a read.
    pub(crate) last_set_mask: u8,
}

impl<I2C, E> PcAsync<I2C>
where
    I2C: I2c<Error = E>,
{
    /// Create new instance of the device
    pub fn new(i2c: I2C, address: SlaveAddr) -> Self {
        let data = PcData {
            i2c,
            address: address.addr(0b010_0000),
            last_set_mask: 0,
        };
        PcAsync {
            data: RefCell::new(data),
        }
    }

    /// Destroy driver instance, return I²C bus instance.
    pub fn destroy(self) -> I2C {
        self.data.into_inner().i2c
    }

    // pub(crate) async fn do_on_acquired<R, Fut>(
    //     &self,
    //     f: impl FnOnce(&Mutex<CriticalSectionRawMutex, PcData<I2C>>) -> Fut,
    // ) -> Result<R, Error<E>>
    // where
    //     Fut: core::future::Future<Output = Result<R, Error<E>>>, // Constraint for async
    // {
    //     let dev = &self.data;
    //     // .map_err(|_| Error::CouldNotAcquireDevice)?;
    //     f(dev).await
    // }

    /// Set the status of all I/O pins.
    pub async fn set(&mut self, bits: u8) -> Result<(), Error<E>> {
        // self.do_on_acquired(|dev| Self::_set(dev, bits)).await
        // self.do_on_acquired(async |dev|elf::_set(dev, bits).await)
        //     .await
        Self::_set(self.data.borrow_mut(), bits).await
    }

    pub(crate) async fn _set(mut dev: RefMut<'_, PcData<I2C>>, bits: u8) -> Result<(), Error<E>> {
        let address = dev.address;
        dev.i2c.write(address, &[bits]).await.map_err(Error::I2C)?;
        dev.last_set_mask = bits;
        Ok(())
    }

    /// Set the status of all I/O pins repeatedly by looping through each array element
    pub async fn write_array(&mut self, data: &[u8]) -> Result<(), Error<E>> {
        if let Some(last) = data.last() {
            // let mut dev = self.data.lock().await;
            let mut dev = self.data.borrow_mut();
            let address = dev.address;
            dev.i2c.write(address, &data).await.map_err(Error::I2C)?;
            dev.last_set_mask = *last;
        }
        Ok(())
    }

    /// Split device into individual pins
    pub fn split(&self) -> pcf8574::Parts<'_, PcAsync<I2C>, E> {
        pcf8574::Parts::new(&self)
    }
}

impl<I2C, E> PcAsync<I2C>
where
    I2C: I2c<Error = E>,
{
    /// Get the status of the selected I/O pins.
    /// The mask of the pins to be read can be created with a combination of
    /// `PinFlag::P0` to `PinFlag::P7`.
    pub async fn get(&mut self, mask: PinFlag) -> Result<u8, Error<E>> {
        if (mask.mask >> 8) != 0 {
            return Err(Error::InvalidInputData);
        }

        // self.do_on_acquired(async |dev| Self::_get(dev, mask).await)
        //     .await
        Self::_get(self.data.borrow_mut(), mask).await
    }

    pub(crate) async fn _get(
        mut dev: RefMut<'_, PcData<I2C>>,
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
    pub async fn read_array(&mut self, mask: PinFlag, mut data: &mut [u8]) -> Result<(), Error<E>> {
        if !data.is_empty() {
            if (mask.mask >> 8) != 0 {
                return Err(Error::InvalidInputData);
            }
            // let mut dev = self.data.lock().await;
            let mut dev = self.data.borrow_mut();
            let mask = mask.mask as u8 | dev.last_set_mask;
            let address = dev.address;
            // configure selected pins as inputs
            dev.i2c.write(address, &[mask]).await.map_err(Error::I2C)?;

            dev.i2c.read(address, &mut data).await.map_err(Error::I2C)?;
        }
        Ok(())
    }
}
