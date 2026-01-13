# FANET ID Format Documentation

## Overview

This document explains how FANET (Flying Ad-hoc NETwork) device IDs are encoded in OGN APRS messages and how they are handled by the SOAR system.

## Background

The issue was raised about a FANET message with what appeared to be an "awfully long" ID:

```
FNT1142BB>OGNAVI,qAS,NAVITER2:/114239h4128.47N/08134.48W'000/000/A=000945 !W59! id18501142BB +000fpm +0.0rot
```

The ID field `id18501142BB` contains **10 hex digits** after the "id" prefix, making it 12 characters total. This initially appeared problematic since standard FLARM/OGN IDs are only 6 hex digits (24-bit addresses).

## ID Format Explanation

The ogn-parser library (which SOAR uses) supports **two ID formats**:

### 1. Standard Format (8 hex digits after "id")

Format: `idXXYYYYYY` (10 characters total)

- `XX`: 1-byte detail field (2 hex digits)
- `YYYYYY`: 6-digit hex address (24-bit address)

**Detail byte structure** (8 bits):
```
Bit 7: S (stealth flag)
Bit 6: T (no-tracking flag)  
Bits 5-2: tttt (aircraft type, 4 bits)
Bits 1-0: aa (address type, 2 bits)
```

**Example**: `id06DDA5BA`
- Detail: `0x06` = `0000 0110` binary
  - Stealth: 0 (false)
  - No-track: 0 (false)
  - Aircraft type: 0001 (1)
  - Address type: 10 (2 = Flarm)
- Address: `0xDDA5BA` = 14,525,882 decimal

### 2. NAVITER Format (10 hex digits after "id")

Format: `idXXXXYYYYYY` (12 characters total)

- `XXXX`: 2-byte detail field (4 hex digits)
- `YYYYYY`: 6-digit hex address (24-bit address)

**Detail field structure** (16 bits):
```
Bit 15: S (stealth flag)
Bit 14: T (no-tracking flag)
Bits 13-10: tttt (aircraft type, 4 bits)
Bits 9-4: aaaaaa (address type, 6 bits)
Bits 3-0: rrrr (reserved, 4 bits)
```

**Example**: `id18501142BB` (from the issue)
- Detail: `0x1850` = `0001 1000 0101 0000` binary
  - Stealth: 0 (false)
  - No-track: 0 (false)
  - Aircraft type: 0001 (1)
  - Address type: 100001 (33)
  - Reserved: 0000 (0)
- Address: `0x1142BB` = 1,131,195 decimal

## Why This Works

The key insight is that **the address is always the last 6 hex digits**, regardless of format. The extra digits in the NAVITER format are **metadata**, not part of the address itself.

### Database Compatibility

SOAR stores device addresses as PostgreSQL `INTEGER` (signed 32-bit), which has a range of:
- Minimum: -2,147,483,648
- Maximum: 2,147,483,647

A 24-bit address (6 hex digits) has a maximum value of:
- Maximum: 0xFFFFFF = 16,777,215

This fits comfortably within the i32 range, so there is **NO overflow problem**.

## Parsing Flow in SOAR

1. **ogn-parser** library parses the APRS message
2. The parser detects the ID format by counting characters after "id"
   - 8 hex digits → Standard format
   - 10 hex digits → NAVITER format
3. The parser extracts:
   - Detail field (1 or 2 bytes depending on format)
   - Address (always last 6 hex digits)
   - Decodes detail bits into structured fields
4. SOAR receives the parsed ID structure with fields:
   - `address`: u32 (the 6-digit hex value)
   - `address_type`: u16 (extracted from detail field)
   - `aircraft_type`: u8 (extracted from detail field)
   - `is_stealth`: bool (extracted from detail field)
   - `is_notrack`: bool (extracted from detail field)
   - `reserved`: Option<u16> (only in NAVITER format)
5. SOAR converts `address` from u32 to i32 for database storage

## Address Type Values

The NAVITER format supports 6-bit address types (0-63), while the standard format only supports 2-bit (0-3).

Common address types:
- 0: Unknown
- 1: ICAO
- 2: FLARM  
- 3: OGN
- 5-63: Extended types (NAVITER format only)

In the example message, the detail field encodes address type 33; its specific semantic meaning is not defined in the referenced public protocol documentation.

## Conclusion

**There is NO bug in the current implementation.** The system correctly:

1. Parses the 10-digit hex ID as NAVITER format
2. Extracts the 6-digit address (`0x1142BB`)
3. Stores it in the database as a 32-bit integer (1,131,195)
4. Preserves all metadata fields (address type, aircraft type, flags)

The confusion arose from seeing the full 10-digit string and assuming it was all part of the address, when in fact only the last 6 digits are the actual device address.

## References

- ogn-parser library: https://github.com/hut8/ogn-parser-rs
- OGN APRS Protocol: http://wiki.glidernet.org/wiki:ogn-flavoured-aprs
- FANET Protocol: https://github.com/glidernet/ogn-aprs-protocol/blob/master/FANET.protocol.txt
