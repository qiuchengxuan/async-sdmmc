pub mod bus;
pub mod read;
pub mod write;

#[cfg(not(feature = "fugit"))]
use core::time::Duration;

use embedded_hal::{digital::v2::OutputPin, timer::CountDown};
#[cfg(feature = "fugit")]
use fugit::NanosDurationU32 as Duration;

use crate::{
    delay::Delay,
    sd::{
        command::{AppCommand, Command, SendInterfaceCondition},
        response::{self, R1Status},
        Card,
    },
};
pub use bus::{BUSError, Bus, Transfer};

impl<E, F, SPI, CS, C> Bus<SPI, CS, C>
where
    SPI: Transfer<Error = E> + Send,
    CS: OutputPin<Error = F> + Send,
    C: CountDown<Time = Duration> + Send,
{
    #[cfg_attr(not(feature = "async"), deasync::deasync)]
    async fn go_idle(&mut self, delay: &mut impl Delay<u8>) -> Result<(), BUSError<E, F>> {
        // SD v1.0 won't be considered
        for _ in 0..32 {
            match self.send_command(Command::GoIdleState).await {
                Ok(r) => match r.r1.has(R1Status::InIdleState) {
                    true => return Ok(()),
                    false => return Err(BUSError::NotIdle),
                },
                Err(BUSError::NoResponse) => (),
                Err(e) => return Err(e),
            }
            delay.delay_ms(10).await;
        }
        Err(BUSError::NoResponse)
    }

    /// Before init, set SPI clock rate between 100KHZ and 400KHZ
    #[cfg_attr(not(feature = "async"), deasync::deasync)]
    pub async fn init(&mut self, mut delay: impl Delay<u8>) -> Result<Card, BUSError<E, F>> {
        // Supply minimum of 74 clock cycles without CS asserted.
        self.deselect()?;
        trace!("Supply 74 clock cycles");
        self.tx(&[0xFF; 10]).await?;

        self.select()?;
        trace!("Go idle");
        self.go_idle(&mut delay).await?;

        trace!("Query version");
        let mut version = 1;
        let r = self.send_command(Command::SendIfCond(SendInterfaceCondition::spi())).await?;
        if !r.r1.has(R1Status::IllegalCommand) {
            version = 2;
            let r7 = response::R7(r.ex);
            if !r7.voltage_accepted() || r7.echo_back_check_pattern() != 0xAA {
                return Err(BUSError::Generic);
            }
        }
        trace!("Version is {}", version);

        trace!("Initialize");
        let mut r1 = response::R1::default();
        for _ in 0..100 {
            r1 = self.send_app_command(AppCommand::SDSendOpCond(version > 1)).await?.r1;
            if !r1.has(R1Status::InIdleState) {
                break;
            }
            delay.delay_ms(10).await;
        }
        if r1.has(R1Status::InIdleState) {
            return Err(BUSError::Generic);
        }

        trace!("Read OCR");
        let mut card = Card::SDSC(version);
        if version > 1 {
            let r = self.send_app_command(AppCommand::ReadOCR).await?;
            let r3 = response::R3(r.ex);
            if r3.card_capacity_status() {
                card = Card::SDHC;
            }
        }
        self.deselect()?;
        self.rx(&mut [0; 1]).await?; // Make MMC/SD release MISO
        Ok(card)
    }
}
