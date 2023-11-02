use anyhow::Result;
use aranet_btle::Aranet4;
use mqttrs::{encode_slice, Connect, Packet, Protocol, Publish, QosPid};
use serde::Serialize;
use std::{io::Write, net::TcpStream};
use tokio::time;

#[derive(Serialize, Debug)]
struct Payload {
    co2: u16,
    temp: f32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let device = aranet_btle::connect().await?;

    let mut stream = TcpStream::connect("127.0.0.1:1883")?;

    // Allocate buffer.
    let mut buf = [0u8; 1024];

    // Encode an MQTT Connect packet.
    let pkt = Packet::Connect(Connect {
        protocol: Protocol::MQTT311,
        keep_alive: 60,
        client_id: "aranet4",
        clean_session: true,
        last_will: None,
        username: None,
        password: None,
    });

    let len = encode_slice(&pkt, &mut buf).unwrap();

    stream.write_all(&buf[..len])?;

    let mut heartbeat = time::interval(time::Duration::from_secs(30));
    let mut read = time::interval(time::Duration::from_secs(60 * 5));

    loop {
        tokio::select! {
           _ = heartbeat.tick() => {
            let pkt = Packet::Pingreq;
            let len = encode_slice(&pkt, &mut buf).unwrap();

            stream.write_all(&buf[..len])?;
           },
           _ = read.tick() => read_sensor_data(&device, &stream).await?
        }
    }

    Ok(())
}

async fn read_sensor_data(device: &Aranet4, mut stream: &TcpStream) -> Result<()> {
    let mut buf = [0u8; 1024];

    let data = device.read_data().await?;

    let payload = Payload {
        co2: data.co2,
        temp: data.temperature,
    };

    let json = serde_json::to_vec(&payload)?;

    let publish = Publish {
        dup: false,
        qospid: QosPid::AtMostOnce,
        retain: false,
        topic_name: "home/aranet4",
        payload: &json,
    };
    let pkt: Packet = publish.into();
    let len = encode_slice(&pkt, &mut buf).unwrap();

    stream.write_all(&buf[..len])?;

    Ok(())
}
