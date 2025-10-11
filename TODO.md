# TODO


- aircraft model details on the devices/[id] page looks terrible on desktop, no need to have every data point on its own line.
- On the login page, the response isn't necessarily json: "Invalid credentials" is the whole response. Make it json just like the other /data endpoints. Also log the reason why the authentication failed (email not verified? wrong password?) But do not display that to the user.
- when adding the flarmnet cargo packege, i get:
Finished `dev` profile [unoptimized + debuginfo] target(s) in 54.20s
     warning: the following packages contain code that will be rejected by a future version of Rust: quick-xml v0.17.2
     note: to see what the problems were, use the option `--future-incompat-report`, or run `cargo report future-incompatibilities --id 1`
so,


Oct 11 00:55:41 supervillain soar[821014]: 2025-10-11T00:55:41.065867Z  WARN soar::flight_tracker: Detected spurious flight bbf16b65-f342-44c7-81ff-e14dab091fb1: duration=19s, altitude_range=Some(6802)ft, max_agl=Some(7038)ft. Deleting
flight. Fix was Fix { id: 65a46df5-1660-437e-a4c8-84840cb9d726, source: "FLRDF2419", destination: "OGFLR7", via: [Some("qAS"), Some("AVX1126")], raw_packet: "FLRDF2419>OGFLR7,qAS,AVX1126:/005539h5333.01N/01009.54Ez040/000/A=000131 !W93!
 id3ADF2419 +040fpm +0.0rot 53.5dB -9.4kHz gps4x4", timestamp: 2025-10-11T00:55:41.009849969Z, latitude: 53.55031666666666, longitude: 10.15905, altitude_msl_feet: Some(131), altitude_agl: None, device_address: 14623769, address_type: F
larm, aircraft_type_ogn: Some(Reserved), flight_number: None, emitter_category: None, registration: None, model: None, squawk: None, ground_speed_knots: Some(0.0), track_degrees: Some(40.0), climb_fpm: Some(40), turn_rate_rot: None, snr
_db: None, bit_errors_corrected: None, freq_offset_khz: None, gnss_horizontal_resolution: Some(4), gnss_vertical_resolution: Some(4), club_id: None, flight_id: Some(bbf16b65-f342-44c7-81ff-e14dab091fb1), unparsed_data: None, device_id:
2a87d13f-93ed-4ee4-9ad5-2ff46e0990f0, received_at: 2025-10-11T00:55:41.009849969Z, lag: Some(0), is_active: false, receiver_id: None, aprs_message_id: None }
Oct 11 00:55:41 supervillain soar[821014]: 2025-10-11T00:55:41.070612Z  INFO soar::flight_tracker: Cleared flight_id from 13 fixes
Oct 11 00:55:41 supervillain soar[821014]: 2025-10-11T00:55:41.072480Z  INFO soar::flight_tracker: Deleted spurious flight bbf16b65-f342-44c7-81ff-e14dab091fb1
Oct 11 00:55:41 supervillain soar[821014]: 2025-10-11T00:55:41.176144Z ERROR soar::fix_processor: Failed to save fix to database for fix: Fix { id: 65a46df5-1660-437e-a4c8-84840cb9d726, source: "FLRDF2419", destination: "OGFLR7", via: [
Some("qAS"), Some("AVX1126")], raw_packet: "FLRDF2419>OGFLR7,qAS,AVX1126:/005539h5333.01N/01009.54Ez040/000/A=000131 !W93! id3ADF2419 +040fpm +0.0rot 53.5dB -9.4kHz gps4x4", timestamp: 2025-10-11T00:55:41.009849969Z, latitude: 53.550316
66666666, longitude: 10.15905, altitude_msl_feet: Some(131), altitude_agl: None, device_address: 14623769, address_type: Flarm, aircraft_type_ogn: Some(Reserved), flight_number: None, emitter_category: None, registration: None, model: N
one, squawk: None, ground_speed_knots: Some(0.0), track_degrees: Some(40.0), climb_fpm: Some(40), turn_rate_rot: None, snr_db: None, bit_errors_corrected: None, freq_offset_khz: None, gnss_horizontal_resolution: Some(4), gnss_vertical_r
esolution: Some(4), club_id: None, flight_id: Some(bbf16b65-f342-44c7-81ff-e14dab091fb1), unparsed_data: None, device_id: 2a87d13f-93ed-4ee4-9ad5-2ff46e0990f0, received_at: 2025-10-11T00:55:41.009849969Z, lag: Some(0), is_active: false,
 receiver_id: None, aprs_message_id: None }
Oct 11 00:55:41 supervillain soar[821014]: cause:insert or update on table "fixes" violates foreign key constraint "fixes_flight_id_fkey"


## Features

-

- /clubs/[id]/flights

Create this frontend page. Require a login. Require that the user logged in be an administrator for this club. Feature a list of aircraft assigned to that club. For the current day (i.e., since midnight local time) display all flights.

- /flights/[id] - add altitude chart below the map. when hovering over (or touching, on mobile) a point (derived from a fix) on the map, display the altitude (both MSL and AGL) at the nearest fix.
