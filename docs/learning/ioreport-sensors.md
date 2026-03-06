# macOS Temperature Sensor Inventory — M1 MacBook Pro

Empirically discovered on this Mac using `hidutil list`, `ioreg`, `sysctl`, and source code analysis.
**Model**: MacBook Pro with M1 Pro (T6000) — 3 CPU complexes (2× E-core + 1× P-core cluster)

---

## 1. Currently Working (SMC via `macsmc` crate)

These T-keys exist on M1 Pro and return valid temperature values:

| SMC Key | Display Name | Typical °C | Notes |
|---------|-------------|-----------|-------|
| TW0P | Airport Proximity | 41–45 | WiFi chip |
| TB0T | Battery | 30 | |
| TB1T | Battery Gas Gauge | 30 | |
| TCPUAVG | CPU Core Average | 55–65 | derived (average of core sensors) |
| TG0D | GPU Cluster 1 | 48–54 | M1 Pro GPU die |
| TG0P | GPU Cluster 2 | 57–64 | M1 Pro GPU die 2 |
| TGAVG | GPU Cluster Average | 49–56 | derived |
| TPCD | Power Manager Die Average | 55–65 | PCH/PMU die |
| Ts0P | Trackpad | 30–31 | palm rest 1 |
| Ts1P | Trackpad Actuator | 28 | palm rest 2 |

---

## 2. SMC Keys That Exist on M1 Pro but Return 0 (Hidden)

These keys are in the catalog but the hardware doesn't populate them:

| SMC Key | Display Name | Why Missing |
|---------|-------------|-------------|
| TM0P | Memory Bank 1 | No physical sensor — unified memory |
| TM1P | Memory Bank 2 | No physical sensor |
| TaLC | Airflow Left | No airflow thermistor on M1 |
| TaRC | Airflow Right | No airflow thermistor |
| Th1H | Heatpipe 1 | No heatpipe sensor on M1 |
| Th2H | Heatpipe 2 | No heatpipe sensor |
| Tm0P | Mainboard | Returns 0 on M1 |
| TTLD | Thunderbolt Left | Returns 0 on M1 |
| TTRD | Thunderbolt Right | Returns 0 on M1 |
| TC0D/TC0F | CPU Die | Intel key, returns 0 on M1 |
| TC0P | CPU Proximity | Intel key, returns 0 on M1 |
| TCGC | CPU Graphics | Intel key, returns 0 on M1 |
| TCSA | CPU System Agent | Returns 0 on M1 |

---

## 3. HID Temperature Sensors (PMU via `AppleSMCKeysEndpoint`)

Discovered via `hidutil list` with `UsagePage=65280 (0xFF00), Usage=5`. The LocationID decodes directly to the 4-byte SMC key.

### PMU Die Temperatures (`tdie*` series → SMC `TP*b`/`TP*l`)

These are die-level thermal readings from the PMU (Power Management Unit):

| hidutil Name | SMC Key(s) | Likely Meaning on M1 Pro |
|-------------|-----------|--------------------------|
| PMU tdie0 | TP0b | CPU die temp — ECPU cluster 0 |
| PMU tdie1 | TP1b, TP1l | CPU die temp — ECPU cluster 1 |
| PMU tdie2 | TP2b, TP2l | CPU die temp — PCPU cluster |
| PMU tdie3 | TP3b | GPU die temp |
| PMU tdie4 | TP4b | GPU die temp |
| PMU tdie5 | TP5b | Unknown subsystem |
| PMU tdie6 | TP6b | Unknown subsystem |
| PMU tdie7 | TP7b | Unknown subsystem |
| PMU tdie8 | TP8b | Unknown subsystem |
| PMU tdie9 | TP9b | Unknown subsystem |
| PMU tdie10 | TPab | Unknown subsystem |

> **Note**: The exact mapping of tdie* → CPU/GPU cluster is not publicly documented by Apple.
> The macmon project (https://github.com/vladkens/macmon) has reverse-engineered these via IOReport correlation.
> Reading actual values requires opening the HID device and reading events — not accessible from ioreg static dump.

### PMU Device Temperatures (`tdev*` series → SMC `TP*d`)

| hidutil Name | SMC Key | Likely Meaning |
|-------------|---------|---------------|
| PMU tdev1 | TP1d | Unknown device temp |
| PMU tdev2 | TP2d | Unknown device temp |
| PMU tdev3 | TP3d | Unknown device temp |
| PMU tdev4 | TP4d | Unknown device temp |
| PMU tdev5 | TP5d | Unknown device temp |
| PMU tdev6 | TP6d | Unknown device temp |
| PMU tdev7 | TP7d | Unknown device temp |
| PMU tdev8 | TP8d | Unknown device temp |

### PMU Thermal Points (`TP*s`, `TP*g` → scaled thermal management values)

| hidutil Name | SMC Key | Likely Meaning |
|-------------|---------|---------------|
| PMU TP0s | TP0s | CPU cluster thermal point (already in TPCD mapping) |
| PMU TP1s | TP1s | CPU cluster thermal point |
| PMU TP2s | TP2s | CPU cluster thermal point |
| PMU TP1g | TP1g | GPU thermal point |
| PMU TP2g | TP2g | GPU thermal point |
| PMU TP3g | TP3g | GPU thermal point |
| PMU tcal | TP0Z | PMU calibration temperature |

### Battery Gas Gauge Temps

| hidutil Name | SMC Key | Meaning |
|-------------|---------|---------|
| gas gauge battery | TG0B, TG0C, TG0H, TG0V, TG1B, TG2B | Battery cell temperatures |

---

## 4. NVMe / SSD Temperature

| Source | SMC Key / HID Name | Notes |
|--------|-------------------|-------|
| AppleANS3NVMeController | TN0n | NVMe SSD temperature — currently mapped via HID as "NAND CH0 temp". Shows N/A in UI because macsmc doesn't read TN0n via SMC. Read via `ioreg -r -c IOHIDEventService -l` instead. |

**How to read**: The `apple_silicon_sensors.rs` already handles this via `normalize_name("nand ch0 temp")` → maps to `"SSD"` key. The value comes from HID events, not static ioreg properties, so it needs the HID event reading path.

---

## 5. Via IOReport (Not Yet Implemented — Requires Entitlement)

These are only accessible via Apple's private `IOReport` / `PerfPowerServicesReader.framework`:

| Sensor | Description | Why It Matters |
|--------|-------------|----------------|
| ECPU Cluster 0 temp | Efficiency core cluster 0 | Per-cluster CPU thermal state |
| ECPU Cluster 1 temp | Efficiency core cluster 1 | Per-cluster CPU thermal state |
| PCPU Cluster temp | Performance core cluster | Per-cluster CPU thermal state |
| ANE temp | Apple Neural Engine | ML workload thermal |
| Thermal pressure level | 0 = nominal, 1 = moderate, 2 = heavy, 3 = tripping | Is Mac throttling? |
| GPU power draw | mW per cluster | Correlates with GPU temp |

**Framework**: `PerfPowerServicesReader.framework` (on macOS 15+, IOReport split into this)
**Requires**: `com.apple.private.iokit.ioreporter` entitlement — not available in sandboxed apps without Apple approval.
**Reference**: https://github.com/vladkens/macmon (Rust implementation)

---

## 6. Reading Strategy Summary

```
Temperature Source            Method                      Status
─────────────────────────────────────────────────────────────────
Classic SMC T* keys          macsmc crate (IOKit FFI)    ✅ Working
HID PMU tdie*/tdev* temps    IOHIDEventService events    ⚠️  Not yet read individually
HID NVMe temp (TN0n)         IOHIDEventService events    ⚠️  Mapped but value unclear
Battery temp                 AppleSmartBattery ioreg     ✅ Working
Per-cluster CPU temps        IOReport (private)          ❌ Needs entitlement
Thermal throttle state       IOReport (private)          ❌ Needs entitlement
```

---

## 7. What to Implement Next (Priority Order)

1. **SSD temperature** — `TN0n` / "NAND CH0 temp" via HID event reading. Already partially plumbed in `apple_silicon_sensors.rs` but needs value extraction from live events, not static dump.

2. **PMU tdie0-2** — `TP0b`, `TP1b`, `TP2b` — likely ECPU0, ECPU1, PCPU cluster temps. Read via HID event subscription or try SMC key directly. Unknown: exact cluster mapping without IOReport cross-reference.

3. **IOReport integration** — For thermal throttle state and exact per-cluster temps. Requires `PerfPowerServicesReader.framework` FFI in Rust + entitlement for distribution. macmon has Rust bindings that could be adapted.

---

## Tools Used for Discovery

```bash
# List all temperature HID sensors with their SMC keys
hidutil list | awk '$5==5 && $4==65280'

# Decode LocationID (4-byte big-endian) → SMC key
python3 -c "n=0x54503062; print(''.join(chr((n>>(i*8))&0xFF) for i in range(3,-1,-1)))"
# → TP0b

# See static HID device info (no live values)
ioreg -r -c IOHIDEventService -l

# Battery temperature (in centidegrees C or 0.1K)
ioreg -r -n AppleSmartBattery -l | grep Temperature
# → "Temperature" = 3042  (= 30.42°C)

# Enumerate IOReport subscribers (no values without API)
ioreg -r -c IOReportHub -l
```
