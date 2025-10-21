/// ICAO <-> Registration (US "N", Canada "C")
///
/// - US block: 0xA00001 ..= 0xADF669
/// - Canada block: 0xC00001 ..= 0xC0CDF8
///
/// ICAO identifiers represented as [u8; 3], big-endian.
use std::num::ParseIntError;

const US_BASE: u32 = 0xA00000;
const US_MAX: u32 = 0xADF669;
const CA_BASE: u32 = 0xC00000;
const CA_MAX: u32 = 0xC0CDF8;
const CA_BLOCKS: [char; 4] = ['F', 'G', 'H', 'I'];
const SKIP_CG: bool = true;

fn u32_to_arr3(x: u32) -> [u8; 3] {
    [(x >> 16) as u8, (x >> 8) as u8, x as u8]
}
fn arr3_to_u32(a: [u8; 3]) -> u32 {
    ((a[0] as u32) << 16) | ((a[1] as u32) << 8) | (a[2] as u32)
}
fn letter_index(c: char) -> Option<u32> {
    if c.is_ascii_uppercase() {
        Some((c as u8 - b'A') as u32)
    } else {
        None
    }
}

// === US implementation ===
fn us_n_to_icao_u32(reg: &str) -> Result<u32, String> {
    let reg = reg.trim().to_ascii_uppercase();
    let mut chars = reg.chars();
    if chars.next() != Some('N') {
        return Err("Must start with N".into());
    }
    let body: String = chars.collect();

    let mut digits = String::new();
    let mut letters = String::new();
    for ch in body.chars() {
        if ch.is_ascii_digit() && letters.is_empty() {
            digits.push(ch);
        } else if ch.is_ascii_alphabetic() {
            letters.push(ch);
        } else {
            return Err(format!("Invalid char {}", ch));
        }
    }
    let numeric: u32 = digits
        .parse::<u32>()
        .map_err(|e: ParseIntError| e.to_string())?;
    if !(1..=999).contains(&numeric) {
        return Err("Numeric part out of range (1–999)".into());
    }

    let per_block = 1 + 26 + 26 * 26;
    let numeric_block_index = (numeric - 1) * per_block;

    let suffix_offset: u32 = match letters.len() {
        0 => 0,
        1 => 1 + letter_index(letters.chars().next().unwrap()).ok_or("bad suffix")?,
        2 => {
            let a = letter_index(letters.chars().next().unwrap()).ok_or("bad suffix")?;
            let b = letter_index(letters.chars().nth(1).unwrap()).ok_or("bad suffix")?;
            1 + 26 + a * 26 + b
        }
        _ => return Err("Suffix too long".into()),
    };

    let index = numeric_block_index + suffix_offset + 1;
    let icao = US_BASE + index;
    if icao > US_MAX {
        return Err("Out of US range".into());
    }
    Ok(icao)
}

fn icao_u32_to_us(icao: u32) -> Result<String, String> {
    if !(US_BASE + 1..=US_MAX).contains(&icao) {
        return Err("Not in US allocation".into());
    }
    let idx = icao - US_BASE;
    let per_block = 1 + 26 + 26 * 26;
    let idx0 = idx - 1;
    let num_block = idx0 / per_block;
    let intra = idx0 % per_block;

    let numeric = num_block + 1;
    let reg = if intra == 0 {
        format!("N{}", numeric)
    } else if intra <= 26 {
        let l = (b'A' + (intra - 1) as u8) as char;
        format!("N{}{}", numeric, l)
    } else {
        let two = intra - 1 - 26;
        let a = (two / 26) as u8;
        let b = (two % 26) as u8;
        format!("N{}{}{}", numeric, (b'A' + a) as char, (b'A' + b) as char)
    };
    Ok(reg)
}

// === Canada implementation ===
fn canada_to_icao_u32(reg: &str) -> Result<u32, String> {
    let reg = reg.trim().to_ascii_uppercase();
    if !reg.starts_with('C') {
        return Err("Must start with C".into());
    }
    let rest = reg
        .strip_prefix("C-")
        .or_else(|| reg.strip_prefix('C'))
        .ok_or("Must start with C")?;
    if rest.len() != 4 {
        return Err("Expect C + 4 letters".into());
    }
    let mut it = rest.chars();
    let block = it.next().unwrap();
    if SKIP_CG && block == 'G' {
        return Err("CG skipped".into());
    }
    let idx_block = CA_BLOCKS
        .iter()
        .position(|&b| b == block)
        .ok_or("Bad block")?;
    let a = letter_index(it.next().unwrap()).ok_or("bad A")?;
    let b = letter_index(it.next().unwrap()).ok_or("bad B")?;
    let c = letter_index(it.next().unwrap()).ok_or("bad C")?;
    let per_block = 26 * 26 * 26;
    let three_index = a * 26 * 26 + b * 26 + c;
    let index = (idx_block as u32) * per_block + three_index + 1;
    let icao = CA_BASE + index;
    if icao > CA_MAX {
        return Err("Out of Canada range".into());
    }
    Ok(icao)
}

fn icao_u32_to_canada(icao: u32) -> Result<String, String> {
    if !(CA_BASE + 1..=CA_MAX).contains(&icao) {
        return Err("Not in Canada allocation".into());
    }
    let idx = icao - CA_BASE;
    let per_block = 26 * 26 * 26;
    let index0 = idx - 1;
    let block_idx = (index0 / per_block) as usize;
    if block_idx >= CA_BLOCKS.len() {
        return Err("Block out of range".into());
    }
    let block = CA_BLOCKS[block_idx];
    if SKIP_CG && block == 'G' {
        return Err("CG skipped".into());
    }
    let three = index0 % per_block;
    let a = (three / (26 * 26)) as u8;
    let b = ((three / 26) % 26) as u8;
    let c = (three % 26) as u8;
    Ok(format!(
        "C-{}{}{}{}",
        block,
        (b'A' + a) as char,
        (b'A' + b) as char,
        (b'A' + c) as char
    ))
}

// === Public API ===

pub fn registration_to_icao(reg: &str) -> Result<[u8; 3], String> {
    if reg.starts_with('N') {
        us_n_to_icao_u32(reg).map(u32_to_arr3)
    } else if reg.starts_with('C') {
        canada_to_icao_u32(reg).map(u32_to_arr3)
    } else {
        Err("Unsupported registration prefix".into())
    }
}

pub fn icao_to_registration(icao: [u8; 3]) -> Result<String, String> {
    let icao_u32 = arr3_to_u32(icao);
    if (US_BASE + 1..=US_MAX).contains(&icao_u32) {
        icao_u32_to_us(icao_u32)
    } else if (CA_BASE + 1..=CA_MAX).contains(&icao_u32) {
        icao_u32_to_canada(icao_u32)
    } else {
        Err("ICAO not in US or Canada range".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn us_roundtrip() {
        let reg = "N456TS";
        let icao = registration_to_icao(reg).unwrap();
        assert_eq!(icao_to_registration(icao).unwrap(), reg);
    }

    #[test]
    fn ca_roundtrip() {
        let reg = "CFAAA";
        let icao = registration_to_icao(reg).unwrap();
        assert_eq!(icao_to_registration(icao).unwrap(), "C-FAAA");
    }

    #[test]
    fn ca_icao_to_canonical_format() {
        // Test CF block (first block)
        let icao_cf = registration_to_icao("CFAAA").unwrap();
        assert_eq!(icao_to_registration(icao_cf).unwrap(), "C-FAAA");

        // Test CH block (third block, skipping CG)
        let icao_ch = registration_to_icao("CHAAA").unwrap();
        assert_eq!(icao_to_registration(icao_ch).unwrap(), "C-HAAA");

        // Test with different letter combinations
        let icao_cf_xyz = registration_to_icao("CFXYZ").unwrap();
        assert_eq!(icao_to_registration(icao_cf_xyz).unwrap(), "C-FXYZ");
    }

    #[test]
    fn ca_24bit_icao_direct_conversion() {
        // Test direct conversion from 24-bit ICAO codes
        // CF block starts at CA_BASE + 1 = 0xC00001
        let cf_aaa_icao = [0xC0, 0x00, 0x01]; // First Canadian ICAO (CFAAA)
        assert_eq!(icao_to_registration(cf_aaa_icao).unwrap(), "C-FAAA");

        // Test a few positions into the CF block
        let cf_abc_icao = registration_to_icao("CFABC").unwrap();
        assert_eq!(icao_to_registration(cf_abc_icao).unwrap(), "C-FABC");
    }

    #[test]
    fn ca_input_formats() {
        // Test that both "CFAAA" and "C-FAAA" input formats work
        let icao1 = registration_to_icao("CFAAA").unwrap();
        let icao2 = registration_to_icao("C-FAAA").unwrap();
        assert_eq!(icao1, icao2);

        // Both should convert to canonical format with dash
        assert_eq!(icao_to_registration(icao1).unwrap(), "C-FAAA");
        assert_eq!(icao_to_registration(icao2).unwrap(), "C-FAAA");
    }

    #[test]
    fn reject_other() {
        assert!(registration_to_icao("G-ABCD").is_err());
        assert!(icao_to_registration([0x00, 0x12, 0x34]).is_err());
    }
}
