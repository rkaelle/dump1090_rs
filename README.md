# dump978_rs
[<img alt="github" src="https://img.shields.io/badge/github-rsadsb/dump978_rs-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/rsadsb/dump978_rs)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/rsadsb/dump978_rs/main.yml?branch=master&style=for-the-badge" height="20">](https://github.com/rsadsb/dump978_rs/actions?query=branch%3Amaster)

Demodulate UAT (Universal Access Transceiver) ADS-B signals from a software defined radio device tuned to 978 MHz and forward the decoded messages to applications.

This is a complete Rust implementation of a dump978 UAT decoder, supporting:
- 978 MHz UAT ADS-B message decoding
- TIS-B (Traffic Information Service-Broadcast) messages
- FIS-B (Flight Information Service-Broadcast) messages
- Reed-Solomon error correction
- Multiple SDR backends via SoapySDR

## UAT vs Mode S

UAT (Universal Access Transceiver) operates on 978 MHz and is used primarily in the United States for ADS-B messages from aircraft operating below 18,000 feet. It differs from Mode S (1090 MHz) in several key ways:

| Feature | UAT (978 MHz) | Mode S (1090 MHz) |
|---------|---------------|-------------------|
| Frequency | 978 MHz | 1090 MHz |
| Sample Rate | 2.083334 MHz | 2.4 MHz |
| Modulation | CPFSK | PPM |
| Error Correction | Reed-Solomon | Simple CRC |
| Message Types | ADS-B, TIS-B, FIS-B | ADS-B only |
| Typical Use | US domestic < 18,000 ft | International, all altitudes |

## Tested Support

Through the use of the [rust-soapysdr](https://github.com/kevinmehall/rust-soapysdr) project,
we support [many different](https://github.com/pothosware/SoapySDR/wiki) software defined radio devices.
If you have tested this project on devices not listed below, let me know!

| Device                | Supported/Tested | Recommend | argument          |
| --------------------- | :--------------: | :-------: | ----------------- |
| rtlsdr                |        x         |     x     | `--driver rtlsdr` |
| HackRF                |        x         |           | `--driver hackrf` |
| uhd(USRP)             |        x         |           | `--driver uhd`    |
| bladeRF 2.0 micro xA4 | x                |           | `--driver bladerf`|

## Usage
**Minimum Supported Rust Version**: 1.74.0

## Build

Install `soapysdr` drivers and library and `libclang-dev`.

### Note
Using `debug` builds will result in SDR overflows, always use `--release` for production.

### Ubuntu
```bash
sudo apt install libsoapysdr-dev libclang-dev
```

### Cross Compile
Use cross-rs for cross compiling to different architectures:
```bash
cargo install cross
cross build --workspace --target x86_64-unknown-linux-gnu --release

# Used for example in Raspberry Pi (raspios) 32 bit
cross build --workspace --target armv7-unknown-linux-gnueabihf --release

# Used for example in Raspberry Pi (raspios) 64 bit
cross build --workspace --target aarch64-unknown-linux-gnu --release
```

## Run
Run the software using the default rtlsdr tuned to 978 MHz:
```bash
cargo run --release
```

### Help

See `--help` for detailed information:
```
UAT 978 MHz ADS-B Demodulator and Server

Usage: dump978_rs [OPTIONS]

Options:
      --host <HOST>                    ip address to bind with for client connections [default: 127.0.0.1]
      --port <PORT>                    port to bind with for client connections [default: 30978]
      --driver <DRIVER>                soapysdr driver name (sdr device) from default `config.toml` or `--custom-config` [default: rtlsdr]
      --driver-extra <DRIVER_EXTRA>    specify extra values for soapysdr driver specification
      --custom-config <CUSTOM_CONFIG>  Filepath for config.toml file overriding or adding sdr config values for soapysdr
      --enable-fec                     enable Reed-Solomon error correction
      --verbose                        show detailed UAT message information
      --quiet                          don't display hex output of messages
  -h, --help                           Print help (see more with '--help')
  -V, --version                        Print version
```

### Examples

Basic usage with RTL-SDR:
```bash
./dump978_rs --driver rtlsdr
```

With verbose output and FEC enabled:
```bash
./dump978_rs --driver rtlsdr --enable-fec --verbose
```

Listen on a different port:
```bash
./dump978_rs --driver rtlsdr --port 30979
```

## Message Types

dump978_rs supports three types of UAT messages:

### ADS-B Messages
Standard ADS-B messages containing aircraft position, velocity, and identification information.

### TIS-B Messages
Traffic Information Service-Broadcast messages that provide surveillance data about non-UAT-equipped aircraft.

### FIS-B Messages
Flight Information Service-Broadcast messages containing weather data, NOTAMs, and other flight information.

## Output Format

By default, dump978_rs outputs messages in a format compatible with existing ADS-B tools:
```
*5D4CA4658DC2C7864AC96F;
```

With `--verbose` flag, it provides detailed message information:
```
UAT ADS-B: ICAO=A12345 Lat=40.123456 Lon=-74.123456 Alt=5000 Call=N12345
UAT TIS-B: Site=1 Addr=A67890 Lat=40.234567 Lon=-74.234567 Alt=3000
UAT FIS-B: Product=413 TTL=15 Status=0 Data=1024bytes
```

## Configuration

The `config.toml` file contains SDR-specific settings optimized for 978 MHz UAT reception. Key differences from 1090 MHz configuration:

- Higher gain settings may be needed due to weaker UAT signals
- Optimized for 978 MHz frequency
- 2.083334 MHz sample rate
- Antenna configurations suitable for 978 MHz

## Performance Tips

To enable maximum performance, instruct rustc to use features specific to your cpu:
```bash
RUSTFLAGS="-C target-cpu=native" cargo run --release
```

Use the latest Rust releases for best performance (typically 5-10% improvement).

## Testing
```bash
cargo test --workspace --release
```

## Benchmarks
```bash
RUSTFLAGS="-C target-cpu=native" cargo bench --workspace
```

## Technical Details

### UAT Frame Structure
- **Sync Word**: 32-bit synchronization pattern
- **Frame Type**: Uplink or Downlink
- **Payload**: 18-424 bytes depending on message type
- **FEC**: Reed-Solomon error correction

### Demodulation
dump978_rs implements CPFSK (Continuous Phase Frequency Shift Keying) demodulation for UAT signals, which is more robust than the PPM used in Mode S.

### Error Correction
Reed-Solomon error correction is implemented to recover from transmission errors, providing better reliability than simple CRC checking.

## License

This project is licensed under the same terms as the original dump1090_rs project.

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## Changes from dump1090_rs

This project is a complete conversion of dump1090_rs to handle UAT 978 MHz signals:

- Frequency changed from 1090 MHz to 978 MHz
- Sample rate changed from 2.4 MHz to 2.083334 MHz
- Demodulation changed from PPM to CPFSK
- Error correction changed from CRC to Reed-Solomon
- Message format changed from Mode S to UAT
- Added support for TIS-B and FIS-B messages
- Default port changed from 30002 to 30978
