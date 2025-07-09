mod sdrconfig;

use std::io::Write;
use std::net::{IpAddr, TcpListener};

use clap::Parser;
use libdump978_rs::demod_978::demodulate978;
use libdump978_rs::uat_decode::decode_uat_frames;
use libdump978_rs::utils;
use num_complex::Complex;
use sdrconfig::{SdrConfig, DEFAULT_CONFIG};
use soapysdr::Direction;

const DIRECTION: Direction = Direction::Rx;

const CUSTOM_CONFIG_HELP: &str =
    "Filepath for config.toml file overriding or adding sdr config values for soapysdr";
const CUSTOM_CONFIG_LONG_HELP: &str = r#"Filepath for config.toml file overriding or adding sdr config values for soapysdr

An example of overriding the included config of `config.toml` for the rtlsdr:

[[sdr]]
driver = "rtlsdr"

[[sdrs.setting]]
key = "biastee"
value = "true"

[[sdr.gain]]
key = "GAIN"
value = 20.0
"#;

#[derive(Debug, Parser)]
#[clap(
    version,
    name = "dump978_rs",
    author = "wcampbell0x2a",
    about = "UAT 978 MHz ADS-B Demodulator and Server"
)]
struct Options {
    /// ip address to bind with for client connections
    #[clap(long, default_value = "127.0.0.1")]
    host: IpAddr,

    /// port to bind with for client connections
    #[clap(long, default_value = "30978")]
    port: u16,

    /// soapysdr driver name (sdr device) from default `config.toml` or `--custom-config`
    ///
    /// This is used both for instructing soapysdr how to find the sdr and what sdr is being used,
    /// as well as the key value in the `config.toml` file. This must match exactly with the
    /// `.driver` field in order for this application to use the provided config settings.
    #[clap(long, default_value = "rtlsdr")]
    driver: String,

    /// specify extra values for soapysdr driver specification
    #[clap(long)]
    driver_extra: Vec<String>,

    #[clap(long, help = CUSTOM_CONFIG_HELP, long_help = CUSTOM_CONFIG_LONG_HELP)]
    custom_config: Option<String>,

    /// don't display hex output of messages
    #[clap(long)]
    quiet: bool,

    /// enable Reed-Solomon error correction
    #[clap(long)]
    enable_fec: bool,

    /// show detailed UAT message information
    #[clap(long)]
    verbose: bool,
}

// main will exit as 0 for success, 1 on error
fn main() {
    // read in default compiled config
    let mut config: SdrConfig = toml::from_str(DEFAULT_CONFIG).unwrap();

    // parse opts
    let options = Options::parse();

    // parse config from custom filepath
    if let Some(config_filepath) = options.custom_config {
        let custom_config: SdrConfig =
            toml::from_str(&std::fs::read_to_string(&config_filepath).unwrap()).unwrap();
        println!("[-] read in custom config: {config_filepath}");
        // push new configs to the front, so that the `find` method finds these first
        for sdr in custom_config.sdrs {
            config.sdrs.insert(0, sdr);
        }
    }

    // setup soapysdr driver
    let mut driver = String::new();
    driver.push_str(&format!("driver={}", options.driver));

    for e in options.driver_extra {
        driver.push_str(&format!(",{e}"));
    }

    println!("[-] using soapysdr driver_args: {driver}");
    let d = match soapysdr::Device::new(&*driver) {
        Ok(d) => d,
        Err(e) => {
            println!("[!] soapysdr error: {e}");
            return;
        }
    };

    // check if --driver exists in config, with selected driver
    let channel = if let Some(sdr) = config.sdrs.iter().find(|a| a.driver == options.driver) {
        println!("[-] using config: {sdr:#?}");
        // set user defined config settings
        let channel = sdr.channel;

        for gain in &sdr.gain {
            println!("[-] Writing gain: {} = {}", gain.key, gain.value);
            d.set_gain_element(DIRECTION, channel, &*gain.key, gain.value).unwrap();
        }
        if let Some(setting) = &sdr.setting {
            for setting in setting {
                println!("[-] Writing setting: {} = {}", setting.key, setting.value);
                d.write_setting(&*setting.key, &*setting.value).unwrap();
                println!(
                    "[-] Reading setting: {} = {}",
                    setting.key,
                    d.read_setting(&*setting.key).unwrap()
                );
            }
        }

        if let Some(antenna) = &sdr.antenna {
            println!("setting antenna: {}", antenna.name);
            d.set_antenna(DIRECTION, channel, antenna.name.clone()).unwrap();
        }

        // Set frequency to 978 MHz for UAT
        d.set_frequency(DIRECTION, channel, 978_000_000.0, ()).unwrap();
        println!("[-] frequency: {:?}", d.frequency(DIRECTION, channel));

        // Set sample rate to 2.083334 MHz for UAT
        d.set_sample_rate(DIRECTION, channel, 2_083_334.0).unwrap();
        println!("[-] sample rate: {:?}", d.sample_rate(DIRECTION, 0));
        channel
    } else {
        panic!("[-] selected --driver gain values not found in custom or default config");
    };

    let mut stream = d.rx_stream::<Complex<i16>>(&[channel]).unwrap();

    let mut buf = vec![Complex::new(0, 0); stream.mtu().unwrap()];
    stream.activate(None).unwrap();

    // bind to listener port
    let listener = TcpListener::bind((options.host, options.port)).unwrap();
    listener.set_nonblocking(true).expect("Cannot set non-blocking");

    println!("[-] UAT 978 MHz receiver listening on {}:{}", options.host, options.port);

    let mut sockets = vec![];
    let mut frame_count = 0u64;
    let mut message_count = 0u64;

    loop {
        // add more clients
        if let Ok((s, addr)) = listener.accept() {
            println!("[-] client connected from: {}", addr);
            sockets.push(s);
        }

        // try and read from sdr device
        match stream.read(&mut [&mut buf], 5_000_000) {
            Ok(len) => {
                // demodulate new UAT data
                let buf = &buf[..len];
                let outbuf = utils::to_mag(buf);
                
                // Demodulate UAT frames
                let uat_frames = match demodulate978(&outbuf) {
                    Ok(frames) => frames,
                    Err(e) => {
                        if options.verbose {
                            println!("[!] demodulation error: {}", e);
                        }
                        continue;
                    }
                };

                frame_count += uat_frames.len() as u64;

                if !uat_frames.is_empty() {
                    // Decode UAT frames into messages
                    let messages = decode_uat_frames(&uat_frames);
                    message_count += messages.len() as u64;

                    // Format and send messages to connected clients
                    let formatted_messages: Vec<String> = messages
                        .iter()
                        .map(|msg| {
                            let output = if options.verbose {
                                msg.to_hex_string()
                            } else {
                                // For compatibility with existing tools, output hex format
                                match &msg.payload {
                                    libdump978_rs::uat_message::UatMessagePayload::Raw(data) => {
                                        format!("*{};", hex::encode(data))
                                    }
                                    _ => msg.to_hex_string(),
                                }
                            };
                            
                            if !options.quiet {
                                println!("{}", output);
                            }
                            format!("{}\n", output)
                        })
                        .collect();

                    // Send to all connected clients
                    let mut remove_indexes = vec![];
                    for (i, socket) in sockets.iter_mut().enumerate() {
                        for msg in &formatted_messages {
                            if let Err(e) = socket.write_all(msg.as_bytes()) {
                                if e.kind() == std::io::ErrorKind::ConnectionReset {
                                    println!("[-] client disconnected");
                                    remove_indexes.push(i);
                                    break;
                                }
                            }
                        }
                    }

                    // Remove disconnected clients
                    for &i in remove_indexes.iter().rev() {
                        sockets.remove(i);
                    }

                    // Print statistics periodically
                    if frame_count % 1000 == 0 && frame_count > 0 {
                        println!("[-] processed {} frames, {} messages", frame_count, message_count);
                    }
                }
            }
            Err(e) => {
                // exit on sdr timeout
                let code = e.code;
                if matches!(code, soapysdr::ErrorCode::Timeout) {
                    println!("[!] exiting: could not read SDR device");
                    // exit with error code as 1 so that systemctl can restart
                    std::process::exit(1);
                }
            }
        }
    }
}
