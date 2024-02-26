use std::time::Duration;

use dht11::Dht11;
use esp_idf_hal::delay::{Ets, FreeRtos};
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::sys::EspError;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::mqtt::client::{EspMqttClient, EspMqttConnection, MqttClientConfiguration, QoS};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::wifi::{ClientConfiguration, Configuration};
use log::info;

mod smart_home;
const SSID: &str = "PLAY_Swiatlowodowy_AEDE";
const PASSWORD: &str = "Y2JfMnGaxv";

const MQTT_URL: &str = "mqtt://broker.emqx.io:1883";
const MQTT_CLIENT_ID: &str = "esp-mqtt-demo-yalantis";
const MQTT_TOPIC: &str = "esp-mqtt-demo-yalantis";

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    connect_wifi(&mut wifi)?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    info!("WiFi DHCP info: {:?}", ip_info);

    let (mut client, mut conn) = mqtt_create(MQTT_URL, MQTT_CLIENT_ID).unwrap();
    let dht11_pin = PinDriver::input_output_od(peripherals.pins.gpio23).unwrap();
    let mut dht11 = Dht11::new(dht11_pin);
    //run(&mut client, &mut conn, MQTT_TOPIC).unwrap();
    std::thread::Builder::new()
        .stack_size(6000)
        .spawn(move || {
            info!("MQTT Listening for messages");

            while let Ok(event) = conn.next() {
                info!("[Queue] Event: {}", event.payload());
            }

            info!("Connection closed");
        })
        .unwrap();

    client.subscribe(MQTT_TOPIC, QoS::AtMostOnce)?;

    info!("Subscribed to topic \"{MQTT_TOPIC}\"");

    // Just to give a chance of our connection to get even the first published message
    std::thread::sleep(Duration::from_millis(500));
    loop {
        let smart_home = smart_home::read_data(&mut dht11).unwrap_or_default();
        info!("Smart Home Info: {:?}", smart_home);
        let payload = format!("Data:{:?}", smart_home);
        client.publish(MQTT_TOPIC, QoS::AtMostOnce, false, payload.as_bytes())?;

        info!("Published \"{payload}\" to topic \"{MQTT_TOPIC}\"");

        let sleep_secs = 2;

        info!("Now sleeping for {sleep_secs}s...");
        std::thread::sleep(Duration::from_secs(sleep_secs));
    }
    Ok(())
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        bssid: None,
        auth_method: esp_idf_svc::wifi::AuthMethod::WPA2Personal,
        password: PASSWORD.try_into().unwrap(),
        channel: None,
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start()?;

    info!("WiFi Started");

    wifi.connect()?;
    info!("WiFi Connected");

    wifi.wait_netif_up()?;
    info!("WiFi netif up");

    Ok(())
}

fn run(
    client: &mut EspMqttClient<'_>,
    connection: &mut EspMqttConnection,
    topic: &str,
) -> Result<(), EspError> {
    std::thread::scope(|s| {
        info!("About to start the MQTT client");

        // Need to immediately start pumping the connection for messages, or else subscribe() and publish() below will not work
        // Note that when using the alternative constructor - `EspMqttClient::new_cb` - you don't need to
        // spawn a new thread, as the messages will be pumped with a backpressure into the callback you provide.
        // Yet, you still need to efficiently process each message in the callback without blocking for too long.
        //
        // Note also that if you go to http://tools.emqx.io/ and then connect and send a message to topic
        // "esp-mqtt-demo", the client configured here should receive it.
        std::thread::Builder::new()
            .stack_size(6000)
            .spawn_scoped(s, move || {
                info!("MQTT Listening for messages");

                while let Ok(event) = connection.next() {
                    info!("[Queue] Event: {}", event.payload());
                }

                info!("Connection closed");
            })
            .unwrap();

        client.subscribe(topic, QoS::AtMostOnce)?;

        info!("Subscribed to topic \"{topic}\"");

        // Just to give a chance of our connection to get even the first published message
        std::thread::sleep(Duration::from_millis(500));

        let payload = "Hello from esp-mqtt-demo!";

        loop {
            client.enqueue(topic, QoS::AtMostOnce, false, payload.as_bytes())?;

            info!("Published \"{payload}\" to topic \"{topic}\"");

            let sleep_secs = 2;

            info!("Now sleeping for {sleep_secs}s...");
            std::thread::sleep(Duration::from_secs(sleep_secs));
        }
    })
}

fn mqtt_create(
    url: &str,
    client_id: &str,
) -> Result<(EspMqttClient<'static>, EspMqttConnection), EspError> {
    let (mqtt_client, mqtt_conn) = EspMqttClient::new(
        url,
        &MqttClientConfiguration {
            client_id: Some(client_id),
            ..Default::default()
        },
    )?;

    Ok((mqtt_client, mqtt_conn))
}
