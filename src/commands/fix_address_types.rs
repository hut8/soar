use anyhow::{Context, Result};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sql_types::{Integer, Nullable, Text, Uuid as DieselUuid};
use tracing::{info, warn};

use soar::schema::aircraft;

type PgPool = Pool<ConnectionManager<PgConnection>>;

/// One-time fix for aircraft address misidentification.
///
/// Aircraft created spontaneously from live APRS data had their address routed
/// to `other_address` instead of the correct typed column (icao_address,
/// flarm_address, ogn_address) because the address_type parsed from the OGN ID
/// field was discarded in favor of a tracker_device_type-based heuristic.
///
/// This command:
/// 1. Finds all aircraft with `other_address IS NOT NULL`
/// 2. Finds a raw APRS message for each via the fixes table
/// 3. Re-parses the OGN ID field to extract the correct address_type
/// 4. Moves the address to the correct column
pub async fn handle_fix_address_types(pool: &PgPool, dry_run: bool) -> Result<()> {
    info!("Starting address type fix (dry_run={})", dry_run);

    let mut conn = pool.get().context("Failed to get database connection")?;

    // Find all aircraft with other_address set, along with a sample raw message
    // We use a lateral join to efficiently get one raw message per aircraft
    let affected_aircraft: Vec<(
        uuid::Uuid,     // aircraft.id
        i32,            // other_address
        Option<String>, // tracker_device_type
        Option<String>, // raw_message text
    )> = diesel::sql_query(
        r#"
        SELECT
            a.id,
            a.other_address,
            a.tracker_device_type,
            rm.msg
        FROM aircraft a
        LEFT JOIN LATERAL (
            SELECT encode(rm.raw_message, 'escape') as msg
            FROM fixes f
            JOIN raw_messages rm ON rm.id = f.raw_message_id AND rm.received_at = f.received_at
            WHERE f.aircraft_id = a.id
            LIMIT 1
        ) rm ON true
        WHERE a.other_address IS NOT NULL
        ORDER BY a.id
        "#,
    )
    .load::<AircraftWithMessage>(&mut conn)
    .context("Failed to query aircraft with other_address")?
    .into_iter()
    .map(|r| (r.id, r.other_address, r.tracker_device_type, r.msg))
    .collect();

    info!(
        "Found {} aircraft with other_address set",
        affected_aircraft.len()
    );

    let mut moved_to_icao = 0u32;
    let mut moved_to_flarm = 0u32;
    let mut moved_to_ogn = 0u32;
    let mut kept_as_other = 0u32;
    let mut no_message = 0u32;
    let mut no_id_field = 0u32;
    let mut conflicts = 0u32;

    for (aircraft_id, address, tracker_device_type, raw_msg) in &affected_aircraft {
        let Some(msg) = raw_msg else {
            no_message += 1;
            continue;
        };

        // Parse the address_type from the OGN ID field in the raw message
        let Some(address_type) = extract_address_type_from_message(msg) else {
            no_id_field += 1;
            continue;
        };

        // Map to our AddressType enum values
        let target_column = match address_type {
            0 => {
                kept_as_other += 1;
                continue; // Already correctly in other_address
            }
            1 => "icao",
            2 => "flarm",
            3 => "ogn",
            _ => {
                // Extended address types (NAVITER) → keep as other
                kept_as_other += 1;
                continue;
            }
        };

        // Check for conflicts in both dry-run and real mode so counts are accurate
        let conflict = match target_column {
            "icao" => aircraft::table
                .filter(aircraft::icao_address.eq(*address))
                .filter(aircraft::id.ne(*aircraft_id))
                .select(aircraft::id)
                .first::<uuid::Uuid>(&mut conn)
                .optional()
                .context("Failed to check for icao_address conflict")?,
            "flarm" => aircraft::table
                .filter(aircraft::flarm_address.eq(*address))
                .filter(aircraft::id.ne(*aircraft_id))
                .select(aircraft::id)
                .first::<uuid::Uuid>(&mut conn)
                .optional()
                .context("Failed to check for flarm_address conflict")?,
            "ogn" => aircraft::table
                .filter(aircraft::ogn_address.eq(*address))
                .filter(aircraft::id.ne(*aircraft_id))
                .select(aircraft::id)
                .first::<uuid::Uuid>(&mut conn)
                .optional()
                .context("Failed to check for ogn_address conflict")?,
            _ => unreachable!(),
        };

        if let Some(conflicting_id) = conflict {
            warn!(
                "Conflict: aircraft {} has addr {:06X} in other_address, but aircraft {} already has it in {}_address. Skipping.",
                aircraft_id, address, conflicting_id, target_column
            );
            conflicts += 1;
            continue;
        }

        if dry_run {
            info!(
                "Would move aircraft {} (addr={:06X}, tracker={:?}) from other_address to {}_address",
                aircraft_id, address, tracker_device_type, target_column
            );
        } else {
            // Move the address: set the target column and clear other_address
            match target_column {
                "icao" => {
                    diesel::update(aircraft::table.filter(aircraft::id.eq(*aircraft_id)))
                        .set((
                            aircraft::icao_address.eq(Some(*address)),
                            aircraft::other_address.eq(None::<i32>),
                        ))
                        .execute(&mut conn)
                        .context("Failed to update aircraft icao_address")?;
                }
                "flarm" => {
                    diesel::update(aircraft::table.filter(aircraft::id.eq(*aircraft_id)))
                        .set((
                            aircraft::flarm_address.eq(Some(*address)),
                            aircraft::other_address.eq(None::<i32>),
                        ))
                        .execute(&mut conn)
                        .context("Failed to update aircraft flarm_address")?;
                }
                "ogn" => {
                    diesel::update(aircraft::table.filter(aircraft::id.eq(*aircraft_id)))
                        .set((
                            aircraft::ogn_address.eq(Some(*address)),
                            aircraft::other_address.eq(None::<i32>),
                        ))
                        .execute(&mut conn)
                        .context("Failed to update aircraft ogn_address")?;
                }
                _ => unreachable!(),
            }
        }

        match target_column {
            "icao" => moved_to_icao += 1,
            "flarm" => moved_to_flarm += 1,
            "ogn" => moved_to_ogn += 1,
            _ => {}
        }
    }

    info!("Address type fix complete:");
    info!("  Moved to icao_address:  {}", moved_to_icao);
    info!("  Moved to flarm_address: {}", moved_to_flarm);
    info!("  Moved to ogn_address:   {}", moved_to_ogn);
    info!("  Kept as other_address:  {}", kept_as_other);
    info!("  No raw message found:   {}", no_message);
    info!("  No ID field in message: {}", no_id_field);
    info!("  Conflicts (skipped):    {}", conflicts);

    if dry_run {
        info!("DRY RUN - no changes were made. Run without --dry-run to apply.");
    }

    Ok(())
}

/// Extract the address_type from an OGN APRS message's ID field.
///
/// Handles both standard format (idXXYYYYYY, 8 hex) and NAVITER format (idXXXXYYYYYY, 10 hex).
fn extract_address_type_from_message(msg: &str) -> Option<u16> {
    // Find the "id" field in the APRS comment
    let id_start = msg.find(" id")? + 3; // skip " id"
    let remaining = &msg[id_start..];

    // Collect hex digits
    let hex_chars: String = remaining
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();

    match hex_chars.len() {
        8 => {
            // Standard format: first 2 hex = detail byte
            // address_type = bits [1:0]
            let detail = u8::from_str_radix(&hex_chars[..2], 16).ok()?;
            Some((detail & 0x03) as u16)
        }
        10 => {
            // NAVITER format: first 4 hex = detail word
            // address_type = bits [9:4]
            let detail = u16::from_str_radix(&hex_chars[..4], 16).ok()?;
            Some((detail >> 4) & 0x3F)
        }
        _ => None,
    }
}

#[derive(QueryableByName)]
struct AircraftWithMessage {
    #[diesel(sql_type = DieselUuid)]
    id: uuid::Uuid,
    #[diesel(sql_type = Integer)]
    other_address: i32,
    #[diesel(sql_type = Nullable<Text>)]
    tracker_device_type: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    msg: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_standard_format() {
        // id0B495F88 → detail=0x0B=0000_1011, address_type bits [1:0] = 11 = 3 (OGN)
        let msg = "OGN495F88>OGNTRK,qAS,E68:/213013h3305.09N/11209.64W'256/000/A=001257 !W80! id0B495F88 +000fpm";
        assert_eq!(extract_address_type_from_message(msg), Some(3));
    }

    #[test]
    fn test_extract_standard_flarm() {
        // id06DDA5BA → detail=0x06=0000_0110, address_type bits [1:0] = 10 = 2 (Flarm)
        let msg = "FLRDDA5BA>OGFLR,qAS,test:/000000h0000.00N/00000.00E'000/000/A=000000 !W00! id06DDA5BA +000fpm";
        assert_eq!(extract_address_type_from_message(msg), Some(2));
    }

    #[test]
    fn test_extract_standard_icao() {
        // id213D323B → detail=0x21=0010_0001, address_type bits [1:0] = 01 = 1 (ICAO)
        let msg = "ICA3D323B>OGFLR7,qAS,test:/000000h0000.00N/00000.00E'000/000/A=000000 !W00! id213D323B +000fpm";
        assert_eq!(extract_address_type_from_message(msg), Some(1));
    }

    #[test]
    fn test_extract_naviter_format() {
        // id1C40FE534F → detail=0x1C40, bits [9:4] = 0001_1100_0100_0000 >> 4 = 0001_1100_0100 & 0x3F = 000100 = 4
        let msg = "NAVFE534F>OGNAVI,qAS,NAVITER2:/000000h0000.00S/00000.00W'000/000/A=000000 !W00! id1C40FE534F +000fpm";
        assert_eq!(extract_address_type_from_message(msg), Some(4));
    }

    #[test]
    fn test_extract_naviter_ogn() {
        // If NAVITER had address_type=3 (OGN), bits [9:4] would be 000011 = 3
        // detail word = 0000_0000_0011_0000 = 0x0030
        let msg = "test>test:/000000h0000.00N/00000.00E'000/000/A=000000 id0030123456 +000fpm";
        assert_eq!(extract_address_type_from_message(msg), Some(3));
    }

    #[test]
    fn test_no_id_field() {
        let msg = "test>test:/000000h0000.00N/00000.00E'000/000/A=000000 +000fpm";
        assert_eq!(extract_address_type_from_message(msg), None);
    }
}
