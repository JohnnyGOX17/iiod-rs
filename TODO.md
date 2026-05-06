# Development To-Do List

A from-scratch Rust implementation of the `iiod` network wire protocol for controlling AD936x-based SDRs (PlutoSDR, FMCOMMS2/3/4/5, ADRV9361/9364, Oxygen SDR, etc.) without depending on `libiio`.

---

## Phase 0: Reserve & Scaffold

- [ ] Reserve crate name: publish `0.0.1` skeleton to crates.io immediately
- [ ] Set up `Cargo.toml` metadata (license = `MIT OR Apache-2.0`, repository, keywords = `["sdr", "iio", "ad9361", "plutosdr", "rf"]`, categories)
- [ ] Create directory structure:
  ```
  src/
    lib.rs          # Public re-exports, top-level docs
    transport.rs    # Transport trait + TcpTransport impl
    protocol.rs     # Command serialization / response parsing
    device.rs       # Device abstraction, channel topology
    buffer.rs       # IQ sample buffer types, zero-copy access
    error.rs        # Thiserror-based error enum
    profile.rs      # Known device profiles (Pluto, FMCOMMS, etc.)
  examples/
    rx_iq.rs        # Minimal RX IQ capture to stdout
    tx_tone.rs      # Single-tone TX example
    scan_attrs.rs   # Enumerate all device attributes
  ```
- [ ] Set up GitHub repo with `main` branch protections
- [ ] Add CI via GitHub Actions: `cargo fmt --check`, `cargo clippy`, `cargo test`, `cargo doc`
- [ ] Add `.github/FUNDING.yml` if desired

---

## Phase 1: Transport Layer

- [ ] Define `Transport` trait:
  ```rust
  pub trait Transport: Send {
      fn send(&mut self, cmd: &[u8]) -> Result<()>;
      fn recv_line(&mut self) -> Result<String>;
      fn recv_exact(&mut self, buf: &mut [u8]) -> Result<()>;
  }
  ```
- [ ] Implement `TcpTransport` (blocking, `std::net::TcpStream`)
  - [ ] Configurable connect timeout
  - [ ] `BufReader` for line-oriented command responses
  - [ ] Raw read path for bulk sample data (bypass `BufReader`)
- [ ] Stub `AsyncTcpTransport` behind a `tokio` feature gate (impl later)
- [ ] Stub `SerialTransport` behind a `serial` feature gate (for UART access)
- [ ] Unit tests: mock transport for protocol tests

---

## Phase 2: Wire Protocol Engine

- [ ] Implement command serializer — each command as an enum variant:
  - [ ] `VERSION`
  - [ ] `PRINT` (list devices/channels/attributes)
  - [ ] `READ <dev> [<chan> [<is_output>]] <attr>`
  - [ ] `WRITE <dev> [<chan> [<is_output>]] <attr> <len>\n<value>`
  - [ ] `OPEN <dev> <samples_count> <cyclic>`
  - [ ] `CLOSE <dev>`
  - [ ] `READBUF <dev> <bytes_count>`
  - [ ] `WRITEBUF <dev> <bytes_count>`
  - [ ] `SETTRIG <dev> <trigger>` (if needed)
  - [ ] `GETTRIG <dev>`
  - [ ] `TIMEOUT <timeout_ms>` (v0.x protocol)
- [ ] Implement response parser:
  - [ ] Numeric return codes (negative = `-errno`)
  - [ ] Length-prefixed binary payloads
  - [ ] Map negative codes to `std::io::ErrorKind` or custom error variants
- [ ] Protocol version negotiation (`VERSION` handshake on connect)
- [ ] Handle v0.x vs v1.x protocol differences (v1 has a binary protocol mode)
- [ ] Integration test: `VERSION` round-trip against a real PlutoSDR

---

## Phase 3: Device Model & Attributes

- [ ] `Context` struct — represents an `iiod` connection:
  - [ ] `Context::connect(addr: &str) -> Result<Self>`
  - [ ] `Context::devices() -> Vec<Device>` (populated via `PRINT`)
- [ ] `Device` struct — named IIO device (e.g., `ad9361-phy`):
  - [ ] `Device::channels() -> Vec<Channel>`
  - [ ] `Device::attr_read(name: &str) -> Result<String>`
  - [ ] `Device::attr_write(name: &str, value: &str) -> Result<()>`
- [ ] `Channel` struct — named channel with direction:
  - [ ] `Channel::attr_read(name: &str) -> Result<String>`
  - [ ] `Channel::attr_write(name: &str, value: &str) -> Result<()>`
  - [ ] `Channel::id()`, `Channel::is_output()`
- [ ] Parse `PRINT` XML response into device/channel/attribute tree
- [ ] Typed convenience methods on AD936x-specific devices:
  - [ ] `set_lo_freq(hz: u64)`, `set_sample_rate(hz: u64)`
  - [ ] `set_rf_bandwidth(hz: u64)`, `set_gain_mode(mode: GainMode)`
  - [ ] `set_gain_db(gain: f64)` (manual gain control)

---

## Phase 4: Sample Buffers & Streaming

- [ ] `IqBuffer<T>` — generic over sample type:
  - [ ] Primary target: `i16` (AD9363 12-bit sign-extended to 16)
  - [ ] Interleaved I/Q layout: `[I0, Q0, I1, Q1, ...]`
  - [ ] `fn as_complex_slice(&self) -> &[(i16, i16)]` (zero-copy reinterpret)
  - [ ] Optional: `fn to_complex_f32(&self) -> Vec<(f32, f32)>` for convenience
- [ ] `RxStream` — continuous RX sample acquisition:
  - [ ] `OPEN` → loop `READBUF` → yield `IqBuffer` chunks
  - [ ] Configurable buffer size (in samples, maps to `OPEN` parameter)
  - [ ] Overflow detection (monitor return codes)
  - [ ] `Iterator` impl for ergonomic `for buf in rx_stream { ... }`
- [ ] `TxStream` — continuous TX sample injection:
  - [ ] `OPEN` with cyclic flag → `WRITEBUF` loop
  - [ ] Support one-shot cyclic buffers (e.g., CW tone, chirp)
- [ ] Benchmark: measure sustained throughput vs. USB 2.0 theoretical max (~4 MSPS complex i16)

---

## Phase 5: Device Profiles

- [ ] `DeviceProfile` enum + associated constants:
  ```rust
  pub enum DeviceProfile {
      PlutoSDR,       // 1x1, AD9363, USB 2.0 gadget Ethernet
      FMCOMMS2,       // 2x2, AD9361, FMC
      FMCOMMS3,       // 2x2, AD9361, FMC (wideband baluns)
      FMCOMMS4,       // 1x1, AD9364, FMC
      FMCOMMS5,       // 4x4, dual AD9361, FMC
      ADRV9361,       // 2x2, AD9361, SoM
      ADRV9364,       // 1x1, AD9364, SoM
      OxygenSDR,      // 2x2, AD9361, USB 3.0 / GbE
      Custom { rx_channels: u8, tx_channels: u8 },
  }
  ```
- [ ] Auto-detect profile from `PRINT` device enumeration when possible
- [ ] Per-profile defaults: default IP, channel names, sample format, max sample rate
- [ ] Document known IP addresses (Pluto: `192.168.2.1`, Oxygen: `192.168.10.1`)

---

## Phase 6: Examples & Documentation

- [ ] `examples/rx_iq.rs` — tune to FM broadcast, capture N samples, write raw IQ to file
- [ ] `examples/tx_tone.rs` — generate single-tone complex exponential, transmit cyclic
- [ ] `examples/scan_attrs.rs` — connect and dump full device/channel/attribute tree
- [ ] `examples/spectrum.rs` — RX + FFT (using `rustfft`) + print ASCII power spectrum
- [ ] Top-level `lib.rs` doc comment with quick-start usage snippet
- [ ] `README.md`:
  - [ ] Badge row (crates.io version, docs.rs, CI status)
  - [ ] 10-line usage example
  - [ ] Tested hardware table
  - [ ] Comparison with `libiio` (when to use which)
  - [ ] Link to `iiod` protocol reference (libiio source)
- [ ] `CONTRIBUTING.md` — how to test without hardware (mock transport)

---

## Phase 7: Pre-Publish Hardening

- [ ] `cargo clippy -- -W clippy::pedantic` clean
- [ ] `cargo fmt` enforced in CI
- [ ] `cargo doc --no-deps` builds warning-free
- [ ] `#![deny(missing_docs)]` on public API
- [ ] `cargo audit` — no known vulnerabilities in deps
- [ ] `cargo publish --dry-run` passes
- [ ] Decide on MSRV (minimum supported Rust version) and document it
- [ ] `CHANGELOG.md` initialized with `## [0.1.0] - Unreleased`
- [ ] Tag `v0.1.0`, push, `cargo publish`

---

## Future / Stretch Goals

- [ ] `async` support via `tokio` feature gate (`AsyncTransport`, `AsyncRxStream`)
- [ ] `#[no_std]` protocol core (decouple from `std::net` for embedded hosts)
- [ ] Multi-device MIMO synchronization helpers (FMCOMMS5 dual-AD9361 cal)
- [ ] DDS (direct digital synthesis) control for TX — `cf-ad9361-dds-core-lpc` attributes
- [ ] FIR filter coefficient upload (`filter_fir_config` attribute)
- [ ] ENSM (Enable State Machine) control helpers (`alert`, `fdd`, `pinctrl`)
- [ ] Register-level debug access (`reg_read` / `reg_write` passthrough)
- [ ] Python bindings via PyO3 (replace `pyadi-iio`'s `libiio` dependency)
- [ ] `tracing` integration for protocol-level debug logging
- [ ] Companion CLI tool: `iiod-rs-cli scan`, `iiod-rs-cli rx --freq 100e6 --rate 2.4e6 -o samples.iq`
