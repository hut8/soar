# FANET ID Issue - Investigation Summary

## TL;DR

**There is NO bug.** The system correctly handles the long FANET ID `id18501142BB`. The extra digits are metadata, not part of the address.

## The Confusion

The APRS message looked like this:
```
FNT1142BB>OGNAVI,qAS,NAVITER2:/114239h4128.47N/08134.48W'000/000/A=000945 !W59! id18501142BB +000fpm +0.0rot
```

At first glance, `id18501142BB` (10 hex digits) seems like it would overflow a 32-bit integer if treated as a single address:
- `0x18501142BB` = 105,589,916,347 (way too large for i32)

But that's **not how it works**!

## How It Actually Works

The ogn-parser library recognizes two ID formats:

### Standard Format: `idXXYYYYYY`
- 2 hex digits for metadata
- 6 hex digits for address

### NAVITER Format: `idXXXXYYYYYY` ← **This is what we have**
- 4 hex digits for metadata  
- 6 hex digits for address

For `id18501142BB`:
- Metadata: `1850` (contains address type, aircraft type, flags)
- **Actual Address: `1142BB`** = 1,131,195 decimal

1,131,195 fits perfectly in an i32 (max: 2,147,483,647). No problem!

## What The Metadata Contains

The `1850` bytes encode:
- Address type: 5 (extended type in NAVITER format)
- Aircraft type: 6
- Stealth flag: false
- No-track flag: false
- Reserved bits: 0

## Verification

I've added:
1. **Documentation** (`docs/FANET_ID_FORMAT.md`) - Complete explanation of both formats
2. **Tests** (`tests/fanet_id_test.rs`) - 4 test cases verifying correct parsing

## Bottom Line

The system already handles this correctly:
✅ Parses the 10-digit ID using NAVITER format
✅ Extracts the 6-digit address correctly
✅ Stores it in the database without overflow
✅ Preserves all metadata fields

No code changes are needed - it's working as designed!
