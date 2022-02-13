pub mod bus;
pub mod read;
pub mod write;

use core::time::Duration;

use embedded_hal::{digital::v2::OutputPin, timer::CountDown};

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
    /// Before init, set SPI clock rate between 100KHZ and 400KHZ
    #[deasync::deasync]
    pub async fn init(&mut self, mut delay: impl Delay<u8>) -> Result<Card, BUSError<E, F>> {
        // Supply minimum of 74 clock cycles without CS asserted.
        self.deselect()?;
        self.tx(&[0xFF; 10]).await?;

        self.select()?;
        // SD v1.0 won't be considered
        let mut attempts = 32;
        while attempts > 0 {
            match self.send_command(Command::GoIdleState).await {
                Ok(r) => match r.r1.has(R1Status::InIdleState) {
                    false => return Err(BUSError::NoResponse),
                    true => break,
                },
                Err(BUSError::NoResponse | BUSError::Command(_)) => attempts -= 1,
                Err(e) => return Err(e),
            }
            delay.delay_ms(10).await;
        }
        if attempts == 0 {
            return Err(BUSError::NoResponse);
        }

        let mut version = 1;
        let r = self.send_command(Command::SendIfCond(SendInterfaceCondition::spi())).await?;
        if !r.r1.has(R1Status::IllegalCommand) {
            version = 2;
            let r7 = response::R7(r.ex);
            if !r7.voltage_accepted() || r7.echo_back_check_pattern() != 0xAA {
                return Err(BUSError::Generic);
            }
        }

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
