# Fan Control on Apple Silicon — Post-Mortem & Learning Resource

**Date**: 2026-03-07
**Hardware**: MacBookPro18,3 (M1 Pro, 14-inch)
**Outcome**: Resolved — fan control working

This document tells the full story of how we debugged and fixed fan speed control on
Apple Silicon. It is written as a learning resource: the journey matters as much as the fix.

---

## Part 1: How Fan Control Works

### The Four Layers

`mac-fan-ctrl` splits fan control into four layers:

1. **Frontend** (Svelte 5) — user picks `auto`, `constant_rpm`, or `sensor_based`
2. **Tauri commands** (`commands.rs`) — translate UI actions into backend calls
3. **FanControlState** (`fan_control.rs`) — stores per-fan configs, runs the control loop
4. **SmcWriter** (`smc_writer.rs`) — performs low-level SMC writes through IOKit FFI

### Key Files

| File | Role |
|------|------|
| `src/lib/tauriCommands.ts` | Frontend API wrappers |
| `src-tauri/src/commands.rs` | Tauri command handlers |
| `src-tauri/src/fan_control.rs` | State management + interpolation loop |
| `src-tauri/src/smc_writer.rs` | Low-level IOKit SMC reads and writes |
| `src-tauri/src/smc.rs` | Sensor reading via `macsmc` crate |
| `src-tauri/src/main.rs` | Polling loop + startup + shutdown |

### Control Flow

When a user sets a constant RPM:

```
Frontend: set_fan_constant_rpm(fan=0, rpm=2000)
  → commands.rs: verify write access, read fan min/max bounds
  → fan_control.rs: store config, call apply_config()
  → smc_writer.rs: unlock → wait for handoff → set mode → write target RPM
  → AppleSMC.kext → RTKit firmware → physical fan
```

When a user sets sensor-based control:

```
Frontend: set_fan_sensor_control(fan=0, sensor="TC0P", low=33, high=85)
  → commands.rs: store SensorBased config
  → main.rs polling loop (every 1s): calls FanControlState::tick()
  → tick(): read sensor temp → interpolate RPM → write via SmcWriter
```

### The Apple Silicon Write Sequence

```
1. Try Ftst=1 (unlock thermalmonitord — key may not exist on M1)
2. Poll F{n}Md until mode leaves System (3) → Auto (0) or Forced (1)
3. Write F{n}Md=1 (force manual mode)
4. Verify mode readback is 0 or 1
5. Write F{n}Tg=<target RPM>
```

### Important SMC Keys

| Key | Type | Description |
|-----|------|-------------|
| `Ftst` | `ui8` | Diagnostic unlock (1=inhibit thermalmonitord) |
| `F{n}Md` | `ui8` | Fan mode: 0=Auto, 1=Forced, 3=System |
| `F{n}Tg` | `flt` / `fpe2` | Target RPM |
| `F{n}Ac` | `flt` / `fpe2` | Actual RPM (read-only) |
| `F{n}Mn` | `flt` / `fpe2` | Min RPM (advisory, not enforced) |
| `F{n}Mx` | `flt` / `fpe2` | Max RPM (advisory, not enforced) |
| `FNum` | `ui8` | Number of fans |

### Intel vs Apple Silicon

On Intel Macs, SMC was a separate chip — direct writes just worked.
On Apple Silicon, SMC is integrated into the SoC with RTKit firmware.
A daemon called `thermalmonitord` continuously enforces thermal policy.
On M3/M4, it uses System Mode (mode 3) which blocks writes.
On M1/M2, it cooperates more but still controls the fan target.

---

## Part 2: The Problem

### Symptom

User sets fan 0 to 3490 RPM. The backend returns `Ok(())`. The UI shows "Constant value
of 3490". But the physical fan stays at ~828 RPM and does not change.

### Initial Log (Before Diagnostic Logging)

```
[cmd] set_fan_constant_rpm: fan_index=0 rpm=3490
[smc_writer] set_fan_target_rpm: fan=0 rpm=3490 bounds=[1200, 5779]
[smc_writer] Ftst key not present (M1) — skipping unlock
[smc_writer] Setting F0Md=1 (forced)
[smc_writer] F0Tg type=flt  size=4
[smc_writer] set_fan_target_rpm: done fan=0 rpm=3490
[cmd] set_fan_constant_rpm result: Ok(())
```

Everything looked successful. No errors. But the fan did not move.

### Why This Was Confusing

The app has two notions of success that are easy to conflate:

1. **Command success**: the SMC IOKit call returned `Ok(())`
2. **Hardware success**: the physical fan RPM changed to the target

Our code only verified the first. The UI made it worse — `overlay_configs()` patches
emitted fan data to show the user's requested target, even when the SMC didn't honor it.
So the UI showed "3490 RPM" while the hardware stayed at 828 RPM.

---

## Part 3: The Investigation

### Step 1 — Initial Hypotheses

We started with three candidates:

| Hypothesis | Reasoning |
|-----------|-----------|
| **thermalmonitord override** | Ftst key absent on M1 Pro → can't inhibit daemon → it overwrites our target |
| **Firmware lockdown** | macOS 14.4 firmware update permanently blocked fan writes on M1/M2 |
| **App not running as root** | SMC writes need UID 0 |

Root was quickly ruled out (already running as root, UID=0, EUID=0).

### Step 2 — Added Diagnostic Logging

We instrumented every layer of the write path:

- **`smc_call()`**: log key name, command type, IOKit result, SMC result, bytes written
- **`unlock_fan_control()`**: read Ftst before/after write, log key type
- **`wait_for_system_mode_handoff()`**: log initial mode, each poll, final mode
- **`set_fan_target_rpm()`**: readback target immediately + after 100ms delay
- **Startup diagnostics**: dump full fan state (modes, RPMs, min/max) on app launch
- **`diagnose_fan_control` command**: callable from frontend for on-demand diagnostics

### Step 3 — First Diagnostic Run

```
[smc_writer] smc_call: key=Ftst cmd=READ_KEYINFO -> SMC result=132 (UNKNOWN KEY)
[smc_writer] Ftst key not present (M1) — skipping unlock
[smc_writer] wait_for_system_mode_handoff: initial mode=0 (Auto)
[smc_writer] Setting F0Md=1 (forced)
[smc_writer] smc_call: key=F0Md cmd=WRITE_BYTES -> OK (wrote 1 bytes: [1])
[smc_writer] verify_mode: fan=0 actual_mode=0 ← wrote 1, read back 0!
[smc_writer] smc_call: key=F0Tg cmd=WRITE_BYTES -> OK (wrote 4 bytes: [68, 250, 0, 0])
[smc_writer] F0Tg readback: wrote=2000 readback=0 diff=2000 ← value GONE instantly
[smc_writer] F0Tg readback after 100ms: 0 ← still gone
[smc_writer] CONFIRMED: thermalmonitord is overriding F0Tg
```

Startup diagnostic also showed anomalies:
```
F0Ac (actual RPM): 0 (raw=[30, 114, 80, 68])  ← non-zero bytes but decode to ~0
F0Mn (min RPM): 0
F0Mx (max RPM): 0
```

Yet `macsmc` crate reads the same keys correctly: min=1200, max=5779.

**Key observations:**
1. SMC accepts writes (IOKit OK, SMC result=0) but data doesn't persist
2. Mode write F0Md=1 reads back as 0 — firmware silently ignores it
3. Our direct SMC reads decode to wrong values, but `macsmc` gets correct values

### Step 4 — Wrong Hypothesis: thermalmonitord Override

We initially concluded that `thermalmonitord` was overriding our writes because:
- Ftst key absent → can't inhibit the daemon
- Writes accepted but don't persist

This seemed plausible until...

### Step 5 — The Breakthrough: Macs Fan Control Works

We tested **Macs Fan Control v1.5.19** on the same machine. It successfully controlled
both fans at "Constant value of 1200" — actual RPM matched the target perfectly.

**This invalidated the thermalmonitord hypothesis.** If the daemon were the blocker,
Macs Fan Control couldn't work either. Our implementation had a bug.

### Step 6 — Wrong Hypothesis: Struct Layout Mismatch

We suspected our `SmcKeyData` struct had wrong alignment, causing data to end up at
incorrect offsets. We compared with the `macsmc` crate source.

**Result: structs are identical** — same fields, same order, same `#[repr(C)]`, same
76-byte size. Struct layout was not the problem.

### Step 7 — The Actual Root Cause

We compared how `macsmc` decodes `flt ` values (`lib.rs:1327`):

```rust
// macsmc crate — the reference that works correctly
b"flt " => return Ok(DataValue::Float(f32::from_ne_bytes(data.try_into()?))),
```

`from_ne_bytes` = **native endian** (little-endian on Apple Silicon ARM).

Our code used `from_be_bytes` / `to_be_bytes` (big-endian).

**This was the bug.** We encoded 2000.0 as `[68, 250, 0, 0]` (big-endian).
The SMC expected `[0, 0, 250, 68]` (little-endian / native on ARM).
The SMC read our bytes as a denormalized float ≈ 0.0 and silently discarded it.

This also explained the read anomalies — we decoded SMC bytes in wrong endian,
so actual RPM `[30, 114, 80, 68]` (which is ~833 RPM in LE) decoded to ~0.0 in BE.

---

## Part 4: The Fix

### Code Change

In `smc_writer.rs`, two functions changed:

**`encode_value()`** — writing RPM to SMC:
```rust
// BEFORE (broken):
b"flt " => Ok(value.to_be_bytes().to_vec()),

// AFTER (working):
b"flt " => Ok(value.to_ne_bytes().to_vec()),
```

**`decode_rpm()`** — reading RPM from SMC:
```rust
// BEFORE (broken):
b"flt " => f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),

// AFTER (working):
b"flt " => f32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
```

Only `flt ` type changed. Integer types (`fpe2`, `sp78`, `ui*`) remain big-endian.

### Verified Working

```
[smc_writer] smc_call: key=F1Tg cmd=WRITE_BYTES -> OK (wrote 4 bytes: [0, 0, 250, 68])
[smc_writer] F1Tg readback after 100ms: 2000 (diff=0 from target 2000)
[smc_writer] set_fan_target_rpm: done fan=1 rpm=2000
```

Fan physically changed speed. Readback confirmed value persists.

---

## Part 5: Why This Bug Was So Hard to Find

### Silent Failure

The SMC returned success (IOKit OK, SMC result=0) for every write. There was no error
code, no exception, no visible failure. The only signal was that the fan didn't move —
which could have many explanations.

### Plausible Wrong Hypothesis

The "thermalmonitord is blocking us" explanation was very convincing:
- Ftst key absent (true)
- Apple Silicon uses thermal enforcement (true)
- macOS 14.4 firmware update locked down fan control (true for some chips)
- Writes don't persist (true, but for the wrong reason)

All the supporting evidence was real. The conclusion was wrong.

### UI Optimism

`overlay_configs()` made the UI show requested values instead of raw SMC readbacks.
This was designed to handle a real Apple Silicon quirk (stale readbacks), but it also
masked the failure — the user saw "3490 RPM" while the hardware ignored us.

### Endianness Is Easy to Get Wrong

The SMC protocol uses **mixed endianness**:
- Integer types (`ui8`, `ui16`, `fpe2`, `sp78`): big-endian (network byte order)
- Float type (`flt `): **native endian** (follows CPU byte order)

On Intel Macs (x86, little-endian), big-endian and the SMC convention happened to be
different, but Intel Macs used `fpe2` for fan RPM, not `flt `. Apple Silicon switched
fan RPM to `flt ` type, and the endianness assumption broke.

---

## Part 6: Endianness Rules for SMC Data Types

| Type | Encoding | Description |
|------|----------|-------------|
| `flt ` | **Native endian** | IEEE 754 float, uses CPU byte order |
| `fpe2` | Big-endian | Unsigned fixed-point (value x 4) |
| `sp78` | Big-endian | Signed fixed-point (value x 256) |
| `ui8 ` | Single byte | No endianness concern |
| `ui16` | Big-endian | Unsigned 16-bit integer |
| `ui32` | Big-endian | Unsigned 32-bit integer |
| `flag` | Single byte | Boolean flag |

Apple Silicon is little-endian (ARM). The SMC protocol's integer types use big-endian.
The `flt ` type is the exception — it follows the CPU's native byte order.

**Reference**: `macsmc` crate source (`lib.rs:1327`):
```rust
b"flt " => return Ok(DataValue::Float(f32::from_ne_bytes(data.try_into()?))),
```

---

## Part 7: M1 Pro Specifics

### Ftst Key Is Absent

```
key=Ftst cmd=READ_KEYINFO -> SMC result=132 (UNKNOWN KEY)
```

The M1 Pro does not have the `Ftst` diagnostic unlock key. This is not a problem — on
M1/M2 chips, `thermalmonitord` does not enforce System Mode (mode 3), so direct writes
to `F{n}Md` and `F{n}Tg` work without unlocking.

### Chip Compatibility

| Chip | Ftst Key | Mode at Idle | Fan Control |
|------|----------|-------------|-------------|
| M1 Pro (MacBookPro18,3) | Absent | 0 or 1 | **Verified working** |
| M1 / M1 Max / M2 | Likely absent | 0 or 1 | Expected to work (same arch) |
| M3 / M3 Pro / M3 Max | Present | 3 (System) | Expected to work with Ftst unlock |
| M4 / M4 Pro / M4 Max | Present | 3 (System) | Expected to work with Ftst unlock |

M3/M4 are untested — the Ftst unlock path is implemented but not hardware-verified.

### Firmware Timeline

```
macOS 12-13      → M1/M2 direct writes worked
macOS 14.0-14.3  → M3 fan control issues surfaced
macOS 14.4       → FIRMWARE UPDATE blocked naive fan control (irreversible)
macOS 15         → Ftst unlock confirmed working (Macs Fan Control v1.5.18)
```

The lockdown is in SMC firmware, not the OS. Downgrading macOS does not restore old firmware.

---

## Part 8: Architecture & Safety

### Current Architecture (Development Mode)

```
┌──────────────────────────────────────────────────┐
│   mac-fan-ctrl (Tauri app, running as root)      │
│                                                   │
│   Frontend (Svelte 5)                             │
│   ├── Sensor display                              │
│   ├── Fan control UI (RPM / auto / presets)       │
│   └── Privilege escalation prompt                 │
│                                                   │
│   Backend (Rust)                                  │
│   ├── SensorService (macsmc crate, read-only)     │
│   ├── SmcWriter (IOKit FFI, read+write)           │
│   ├── FanControlState (per-fan config tracking)   │
│   └── Polling loop (1s, fan control tick)         │
└──────────────────┬───────────────────────────────┘
                   │ IOConnectCallStructMethod
          ┌────────┴────────┐
          │   AppleSMC.kext  │
          └────────┬────────┘
          ┌────────┴────────┐
          │  RTKit Firmware  │
          └─────────────────┘
```

### Target Architecture (Production)

```
┌──────────────────────┐         XPC          ┌──────────────────────────┐
│   mac-fan-ctrl       │  <──────────────>    │  Privileged Helper       │
│   (Tauri app, user)  │                      │  (root daemon)           │
│   - UI + monitoring  │                      │  - SMC writes            │
│   - Preferences      │                      │  - Ftst unlock/lock      │
│   - Sensor polling   │                      │  - Sleep/wake handling   │
└──────────────────────┘                      └──────────────────────────┘
```

Requires Developer ID signing ($99/yr Apple Developer Program) for `SMJobBless`.

### Safety Features

| Feature | Implementation | Status |
|---------|---------------|--------|
| RPM bounds validation | Reject writes outside `[F{n}Mn, F{n}Mx]` | Working |
| Emergency thermal override | All fans max if any sensor >= 95C | Working |
| Mode verification | Readback F{n}Md, reject if mode 3 | Working |
| Write readback | Verify F{n}Tg persists after 100ms | Working |
| Restore on exit | All fans to Auto on window close | Working |
| Restore on Drop | `SmcWriter::drop()` re-locks thermal control | Working |
| System mode handoff | Poll mode up to 10s before writing | Working |

### Known Gaps

- `set_fan_auto()` is not read-back verified
- Sensor-based write failures are silently ignored in the tick loop
- Sleep/wake recovery is not implemented (Apple Silicon may reset state on wake)
- App currently escalates the entire process to root instead of using a privileged helper
- UI overlay can show success when hardware rejected the write

---

## Part 9: What Other Projects Teach Us

| Project | Status | Method |
|---------|--------|--------|
| [Macs Fan Control](https://crystalidea.com/macs-fan-control) (crystalidea) | Working (v1.5.19) | Privileged helper + Ftst unlock |
| [Stats](https://github.com/exelban/stats) (exelban) | Legacy mode | Added Ftst in v2.12.0 |
| [TG Pro](https://www.tunabellysoftware.com/tgpro/) (Tunabelly) | Working | Proprietary privileged helper |
| [macos-smc-fan](https://github.com/agoodkind/macos-smc-fan) (agoodkind) | Research reference | IDA Pro reverse engineering of AppleSMC.kext |
| [smcFanControl](https://github.com/hholtmann/smcFanControl) (hholtmann) | Basic AS support | Standard SMC writes |
| [Asahi Linux](https://asahilinux.org/docs/hw/soc/smc/) | Linux only | Kernel driver (macsmc-hwmon) |

Practical lessons from the ecosystem:
1. Intel and Apple Silicon are different control problems
2. Apple Silicon needs unlock logic, retries, and model-aware behavior
3. Production apps use a privileged helper, not a full GUI app running as root
4. Asahi Linux proves the hardware supports fan control — restrictions are macOS-only

---

## Part 10: Lessons Learned

### For This Project

1. **Always verify hardware effect, not just command success.** The SMC returns OK for
   writes that have no effect. Readback after a delay is essential.

2. **Don't trust the UI overlay.** `overlay_configs()` was designed to improve UX, but it
   masks real failures. Consider showing raw SMC readbacks alongside configured values.

3. **Check the reference implementation for encoding details.** The `macsmc` crate is the
   authoritative source for how to encode/decode SMC data types. Our bug would have been
   caught immediately by reading its `DataValue::convert()` function.

### For Debugging in General

4. **A plausible wrong hypothesis supported by real evidence is the most dangerous kind.**
   The thermalmonitord explanation was logical, well-supported, and wrong. The breakthrough
   came from testing a commercial app on the same hardware — empirical falsification.

5. **When writes "succeed" but have no effect, suspect encoding.** Silent data corruption
   (wrong endianness, wrong struct alignment, wrong type encoding) produces exactly this
   pattern: call succeeds, data is garbage, firmware ignores it.

6. **Mixed endianness is a trap.** When a protocol uses big-endian for integers but native
   endian for floats, it's easy to assume "everything is big-endian" and never question it.

### Rule of Thumb for Contributors

If you change fan-control code, always ask:
1. Did we really gain control of the fan, or only update internal state?
2. What happens if the Mac sleeps, wakes, or the app exits unexpectedly?
3. What is the safe fallback when a write partially succeeds or verification fails?

---

## Sources

- [agoodkind/macos-smc-fan](https://github.com/agoodkind/macos-smc-fan) — Definitive SMC fan control research for Apple Silicon
- [exelban/stats #2928](https://github.com/exelban/stats/issues/2928) — Fan control failure on Apple Silicon
- [exelban/stats v2.12.0](https://github.com/exelban/stats/releases/tag/v2.12.0) — Ftst fix for M3/M4
- [crystalidea/macs-fan-control #733](https://github.com/crystalidea/macs-fan-control/issues/733) — Sonoma 14.4 firmware lockdown
- [crystalidea/macs-fan-control #785](https://github.com/crystalidea/macs-fan-control/issues/785) — M3/M4 Pro/Max blocked
- [Macs Fan Control release notes](https://crystalidea.com/macs-fan-control/release-notes) — v1.5.18 restored M3/M4 control
- [TG Pro fan control blog](https://www.tunabellysoftware.com/blog/files/fan-control-apple-silicon-macs.html)
- [Asahi Linux SMC docs](https://asahilinux.org/docs/hw/soc/smc/) — Confirms hardware supports fan control
- [hholtmann/smcFanControl #126](https://github.com/hholtmann/smcFanControl/issues/126) — M1 Mac support
- [CrystalIDEA: Limited fan control on some models](https://crystalidea.com/macs-fan-control/limited-fan-control-on-some-models)
- [macsmc crate source](https://crates.io/crates/macsmc) — Reference SMC implementation in Rust
