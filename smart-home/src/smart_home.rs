use anyhow::Error;
use dht11::Dht11;
use esp_idf_hal::delay::Ets;
use embedded_hal::digital::v2::InputPin;
use embedded_hal::digital::v2::OutputPin;

#[derive(Debug,Default)]
pub struct Temperature(f32);

#[derive(Debug,Default)]
pub struct Humidity(f32);

#[derive(Debug,Default)]
pub struct SmartHome{
    temp: Temperature,
    humidity: Humidity
}




pub fn read_data<E, T:  InputPin<Error = E> + OutputPin<Error = E>>(sensor: &mut Dht11<T>) -> anyhow::Result<SmartHome>{
        let mut dht11_delay = Ets;
        match sensor.perform_measurement(&mut  dht11_delay){
            Ok(measurement) => {
                Ok(SmartHome {
                    temp: Temperature(measurement.temperature as f32 / 10.0),
                    humidity: Humidity(measurement.humidity as f32 / 10.0)
                })
            },
            Err(e) => Err(Error::msg("message"))
            }
}
