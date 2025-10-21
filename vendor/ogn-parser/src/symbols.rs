//! APRS Symbol definitions and parsing
//!
//! Reference: https://www.aprs.org/symbols/symbolsX.txt

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AprsSymbol {
    // Primary Table (/)
    PoliceSheriff,          // /! BB Police, Sheriff
    ReservedRain,           // /" BC reserved (was rain)
    DigiWhiteCenter,        // /# BD DIGI (white center)
    Phone,                  // /$ BE PHONE
    DxCluster,              // /% BF DX CLUSTER
    HfGateway,              // /& BG HF GATEway
    SmallAircraft,          // /' BH Small AIRCRAFT (SSID-11)
    MobileSatelliteStation, // /( BI Mobile Satellite Station
    WheelchairHandicapped,  // /) BJ Wheelchair (handicapped)
    Snowmobile,             // /* BK SnowMobile
    RedCross,               // /+ BL Red Cross
    BoyScouts,              // /, BM Boy Scouts
    HouseQthVhf,            // /- BN House QTH (VHF)
    X,                      // /. BO X
    RedDot,                 // // BP Red Dot
    CircleObsolete,         // /0 P0 # circle (obsolete)
    TbdNumbered1,           // /1 P1 TBD (these were numbered)
    TbdCirclesPool2,        // /2 P2 TBD (circles like pool)
    TbdBalls3,              // /3 P3 TBD (balls. But with)
    TbdOverlays4,           // /4 P4 TBD (overlays, we can)
    TbdNumbers5,            // /5 P5 TBD (put all #'s on one)
    TbdAvailable6,          // /6 P6 TBD (So 1-9 are available)
    TbdNewUses7,            // /7 P7 TBD (for new uses?)
    TbdMobilesEvents8,      // /8 P8 TBD (They are often used)
    TbdMobilesEvents9,      // /9 P9 TBD (as mobiles at events)
    Fire,                   // /: MR FIRE
    CampgroundPortableOps,  // /; MS Campground (Portable ops)
    Motorcycle,             // /< MT Motorcycle (SSID-10)
    RailroadEngine,         // /= MU RAILROAD ENGINE
    Car,                    // /> MV CAR (SSID-9)
    ServerForFiles,         // /? MW SERVER for Files
    HcFuturePredict,        // /@ MX HC FUTURE predict (dot)
    AidStation,             // /A PA Aid Station
    BbsOrPbbs,              // /B PB BBS or PBBS
    Canoe,                  // /C PC Canoe
    // /D PD (empty in reference)
    EyeballEvents,      // /E PE EYEBALL (Events, etc!)
    FarmVehicleTractor, // /F PF Farm Vehicle (tractor)
    GridSquare6Digit,   // /G PG Grid Square (6 digit)
    HotelBlueBed,       // /H PH HOTEL (blue bed symbol)
    TcpipOnAirNetwork,  // /I PI TcpIp on air network stn
    // /J PJ (empty in reference)
    School,                // /K PK School
    PcUser,                // /L PL PC user (Jan 03)
    MacAprs,               // /M PM MacAPRS
    NtsStation,            // /N PN NTS Station
    Balloon,               // /O PO BALLOON (SSID-11)
    Police,                // /P PP Police
    Tbd,                   // /Q PQ TBD
    RecVehicle,            // /R PR REC. VEHICLE (SSID-13)
    Shuttle,               // /S PS SHUTTLE
    Sstv,                  // /T PT SSTV
    Bus,                   // /U PU BUS (SSID-2)
    Atv,                   // /V PV ATV
    NationalWxServiceSite, // /W PW National WX Service Site
    Helo,                  // /X PX HELO (SSID-6)
    YachtSail,             // /Y PY YACHT (sail) (SSID-5)
    WinAprs,               // /Z PZ WinAPRS
    HumanPerson,           // /[ HS Human/Person (SSID-7)
    TriangleDfStation,     // /\ HT TRIANGLE(DF station)
    MailPostOffice,        // /] HU MAIL/PostOffice(was PBBS)
    LargeAircraft,         // /^ HV LARGE AIRCRAFT
    WeatherStation,        // /_ HW WEATHER Station (blue)
    DishAntenna,           // /` HX Dish Antenna
    Ambulance,             // /a LA AMBULANCE (SSID-1)
    Bike,                  // /b LB BIKE (SSID-4)
    IncidentCommandPost,   // /c LC Incident Command Post
    FireDept,              // /d LD Fire dept
    HorseEquestrian,       // /e LE HORSE (equestrian)
    FireTruck,             // /f LF FIRE TRUCK (SSID-3)
    Glider,                // /g LG Glider
    Hospital,              // /h LH HOSPITAL
    IotaIslandsOnTheAir,   // /i LI IOTA (islands on the air)
    Jeep,                  // /j LJ JEEP (SSID-12)
    Truck,                 // /k LK TRUCK (SSID-14)
    Laptop,                // /l LL Laptop (Jan 03) (Feb 07)
    MicERepeater,          // /m LM Mic-E Repeater
    NodeBlackBullseye,     // /n LN Node (black bulls-eye)
    Eoc,                   // /o LO EOC
    Paraglider,            // /p LP ROVER (puppy, or dog)
    GridSqShownAbove128m,  // /q LQ GRID SQ shown above 128 m
    Repeater,              // /r LR Repeater (Feb 07)
    ShipPwrBoat,           // /s LS SHIP (pwr boat) (SSID-8)
    TruckStop,             // /t LT TRUCK STOP
    Truck18Wheeler,        // /u LU TRUCK (18 wheeler)
    Van,                   // /v LV VAN (SSID-15)
    WaterStation,          // /w LW WATER station
    XAprsUnix,             // /x LX xAPRS (Unix)
    YagiAtQth,             // /y LY YAGI @ QTH
    TbdZ,                  // /z LZ TBD
    // /{ J1 (empty in reference)
    TncStreamSwitch1, // /| J2 TNC Stream Switch
    // /} J3 (empty in reference)
    TncStreamSwitch2, // /~ J4 TNC Stream Switch

    // Alternate Table (\)
    EmergencyAndOverlays,     // \! OBO EMERGENCY (and overlays)
    ReservedAlt,              // \" OC reserved
    OverlayDigiGreenStar,     // \# OD# OVERLAY DIGI (green star)
    BankAtmGreenBox,          // \$ OEO Bank or ATM (green box)
    PowerPlantWithOverlay,    // \% OFO Power Plant with overlay
    IgateRxTx1hop2hop,        // \& OG# I=Igte R=RX T=1hopTX 2=2hopTX
    CrashIncidentSites,       // \' OHO Crash (& now Incident sites)
    CloudyOtherClouds,        // \( OIO CLOUDY (other clouds w ovrly)
    FirenetMeoModisEarthObs,  // \) OJO Firenet MEO, MODIS Earth Obs.
    AvailSnowMoved,           // \* OK AVAIL (SNOW moved to ` ovly S)
    Church,                   // \+ OL Church
    GirlScouts,               // \, OM Girl Scouts
    HouseHfOpPresent,         // \- ONO House (H=HF) (O = Op Present)
    AmbiguousBigQuestionMark, // \. OO Ambiguous (Big Question mark)
    WaypointDestination,      // \/ OP Waypoint Destination
    CircleIrlpEcholinkWires,  // \0 A0# CIRCLE (IRLP/Echolink/WIRES)
    Avail1,                   // \1 A1 AVAIL
    Avail2,                   // \2 A2 AVAIL
    Avail3,                   // \3 A3 AVAIL
    Avail4,                   // \4 A4 AVAIL
    Avail5,                   // \5 A5 AVAIL
    Avail6,                   // \6 A6 AVAIL
    Avail7,                   // \7 A7 AVAIL
    Network80211OrOther,      // \8 A8O 802.11 or other network node
    GasStationBluePump,       // \9 A9 Gas Station (blue pump)
    AvailHail,                // \: NR AVAIL (Hail ==> ` ovly H)
    ParkPicnicOverlayEvents,  // \; NSO Park/Picnic + overlay events
    AdvisoryOneWxFlag,        // \< NTO ADVISORY (one WX flag)
    AvailSymbolOverlayGroup,  // \= NUO avail. symbol overlay group
    OverlayedCarsVehicles,    // \> NV# OVERLAYED CARs & Vehicles
    InfoKioskBlueBox,         // \? NW INFO Kiosk (Blue box with ?)
    HuricanetropStorm,        // \@ NX HURICANE/Trop-Storm
    OverlayBoxDtmfRfidXo,     // \A AA# overlayBOX DTMF & RFID & XO
    AvailBlowingSnow,         // \B AB AVAIL (BlwngSnow ==> E ovly B
    CoastGuard,               // \C AC Coast Guard
    DepotsAndDrizzle,         // \D ADO DEPOTS (Drizzle ==> ' ovly D)
    SmokeAndOtherVisCodes,    // \E AE Smoke (& other vis codes)
    AvailFreezingRain,        // \F AF AVAIL (FrzngRain ==> `F)
    AvailSnowShower,          // \G AG AVAIL (Snow Shwr ==> I ovly S)
    HazeAndOverlayHazards,    // \H AHO \Haze (& Overlay Hazards)
    RainShower,               // \I AI Rain Shower
    AvailLightning,           // \J AJ AVAIL (Lightening ==> I ovly L)
    KenwoodHt,                // \K AK Kenwood HT (W)
    Lighthouse,               // \L AL Lighthouse
    MarsArmyNavyAf,           // \M AMO MARS (A=Army,N=Navy,F=AF)
    NavigationBuoy,           // \N AN Navigation Buoy
    OverlayBalloonRocket,     // \O AO Overlay Balloon (Rocket = \O)
    Parking,                  // \P AP Parking
    Quake,                    // \Q AQ QUAKE
    Restaurant,               // \R ARO Restaurant
    SatellitePacsat,          // \S AS Satellite/Pacsat
    Thunderstorm,             // \T AT Thunderstorm
    Sunny,                    // \U AU SUNNY
    VortacNavAid,             // \V AV VORTAC Nav Aid
    NwsSiteWithOptions,       // \W AW# # NWS site (NWS options)
    PharmacyRxApothicary,     // \X AX Pharmacy Rx (Apothicary)
    RadiosAndDevices,         // \Y AYO Radios and devices
    AvailZ,                   // \Z AZ AVAIL
    WCloudAndHumansOverlay,   // \[ DSO W.Cloud (& humans w Ovrly)
    NewOverlayableGpsSymbol,  // \\ DTO New overlayable GPS symbol
    AvailBackslash,           // \] DU AVAIL
    OtherAircraftOverlays,    // \^ DV# other Aircraft ovrlys (2014)
    WxSiteGreenDigi,          // \_ DW# # WX site (green digi)
    RainAllTypesWithOverlay,  // \` DX Rain (all types w ovrly)
    ArrlAresWinlinkDstar,     // \a SA#O ARRL,ARES,WinLINK,Dstar, etc
    AvailBlowingDustSand,     // \b SB AVAIL(Blwng Dst/Snd => E ovly)
    CdTriangleRacesSatern,    // \c SC#O CD triangle RACES/SATERN/etc
    DxSpotByCallsign,         // \d SD DX spot by callsign
    SleetAndFutureOverlays,   // \e SE Sleet (& future ovrly codes)
    FunnelCloud,              // \f SF Funnel Cloud
    GaleFlags,                // \g SG Gale Flags
    StoreOrHamfest,           // \h SHO Store. or HAMFST Hh=HAM store
    BoxOrPointsOfInterest,    // \i SI# BOX or points of Interest
    WorkZoneSteamShovel,      // \j SJ WorkZone (Steam Shovel)
    SpecialVehicleSuvAtv4x4,  // \k SKO Special Vehicle SUV,ATV,4x4
    AreasBoxCircles,          // \l SL Areas (box,circles,etc)
    ValueSign3DigitDisplay,   // \m SM Value Sign (3 digit display)
    OverlayTriangle,          // \n SN# OVERLAY TRIANGLE
    SmallCircle,              // \o SO small circle
    AvailPartlyCloudy,        // \p SP AVAIL (PrtlyCldy => ( ovly P
    AvailQ,                   // \q SQ AVAIL
    Restrooms,                // \r SR Restrooms
    OverlayShipBoats,         // \s SS# OVERLAY SHIP/boats
    Tornado,                  // \t ST Tornado
    OverlayedTruck,           // \u SU# OVERLAYED TRUCK
    OverlayedVan,             // \v SV# OVERLAYED Van
    FloodingAvalanchesSlides, // \w SWO Flooding (Avalanches/Slides)
    WreckOrObstruction,       // \x SX Wreck or Obstruction ->X<-
    Skywarn,                  // \y SY Skywarn
    OverlayedShelter,         // \z SZ# OVERLAYED Shelter
    AvailFog,                 // \{ Q1 AVAIL? (Fog ==> E ovly F)
    TncStreamSwitchAlt1,      // \| Q2 TNC Stream Switch
    AvailMaybe,               // \} Q3 AVAIL? (maybe)
    TncStreamSwitchAlt2,      // \~ Q4 TNC Stream Switch
}

impl AprsSymbol {
    /// Parse APRS symbol from table and symbol characters
    ///
    /// # Arguments
    /// * `table` - Symbol table character ('/' for primary, '\' for alternate)
    /// * `symbol` - Symbol character
    ///
    /// # Example
    /// ```
    /// use ogn_parser::AprsSymbol;
    ///
    /// let symbol = AprsSymbol::parse('/', '_');
    /// assert_eq!(symbol, Some(AprsSymbol::WeatherStation));
    /// ```
    pub fn parse(table: char, symbol: char) -> Option<Self> {
        match (table, symbol) {
            // Primary table (/)
            ('/', '!') => Some(Self::PoliceSheriff),
            ('/', '"') => Some(Self::ReservedRain),
            ('/', '#') => Some(Self::DigiWhiteCenter),
            ('/', '$') => Some(Self::Phone),
            ('/', '%') => Some(Self::DxCluster),
            ('/', '&') => Some(Self::HfGateway),
            ('/', '\'') => Some(Self::SmallAircraft),
            ('/', '(') => Some(Self::MobileSatelliteStation),
            ('/', ')') => Some(Self::WheelchairHandicapped),
            ('/', '*') => Some(Self::Snowmobile),
            ('/', '+') => Some(Self::RedCross),
            ('/', ',') => Some(Self::BoyScouts),
            ('/', '-') => Some(Self::HouseQthVhf),
            ('/', '.') => Some(Self::X),
            ('/', '/') => Some(Self::RedDot),
            ('/', '0') => Some(Self::CircleObsolete),
            ('/', '1') => Some(Self::TbdNumbered1),
            ('/', '2') => Some(Self::TbdCirclesPool2),
            ('/', '3') => Some(Self::TbdBalls3),
            ('/', '4') => Some(Self::TbdOverlays4),
            ('/', '5') => Some(Self::TbdNumbers5),
            ('/', '6') => Some(Self::TbdAvailable6),
            ('/', '7') => Some(Self::TbdNewUses7),
            ('/', '8') => Some(Self::TbdMobilesEvents8),
            ('/', '9') => Some(Self::TbdMobilesEvents9),
            ('/', ':') => Some(Self::Fire),
            ('/', ';') => Some(Self::CampgroundPortableOps),
            ('/', '<') => Some(Self::Motorcycle),
            ('/', '=') => Some(Self::RailroadEngine),
            ('/', '>') => Some(Self::Car),
            ('/', '?') => Some(Self::ServerForFiles),
            ('/', '@') => Some(Self::HcFuturePredict),
            ('/', 'A') => Some(Self::AidStation),
            ('/', 'B') => Some(Self::BbsOrPbbs),
            ('/', 'C') => Some(Self::Canoe),
            ('/', 'D') => None, // Empty in reference
            ('/', 'E') => Some(Self::EyeballEvents),
            ('/', 'F') => Some(Self::FarmVehicleTractor),
            ('/', 'G') => Some(Self::GridSquare6Digit),
            ('/', 'H') => Some(Self::HotelBlueBed),
            ('/', 'I') => Some(Self::TcpipOnAirNetwork),
            ('/', 'J') => None, // Empty in reference
            ('/', 'K') => Some(Self::School),
            ('/', 'L') => Some(Self::PcUser),
            ('/', 'M') => Some(Self::MacAprs),
            ('/', 'N') => Some(Self::NtsStation),
            ('/', 'O') => Some(Self::Balloon),
            ('/', 'P') => Some(Self::Police),
            ('/', 'Q') => Some(Self::Tbd),
            ('/', 'R') => Some(Self::RecVehicle),
            ('/', 'S') => Some(Self::Shuttle),
            ('/', 'T') => Some(Self::Sstv),
            ('/', 'U') => Some(Self::Bus),
            ('/', 'V') => Some(Self::Atv),
            ('/', 'W') => Some(Self::NationalWxServiceSite),
            ('/', 'X') => Some(Self::Helo),
            ('/', 'Y') => Some(Self::YachtSail),
            ('/', 'Z') => Some(Self::WinAprs),
            ('/', '[') => Some(Self::HumanPerson),
            ('/', '\\') => Some(Self::TriangleDfStation),
            ('/', ']') => Some(Self::MailPostOffice),
            ('/', '^') => Some(Self::LargeAircraft),
            ('/', '_') => Some(Self::WeatherStation),
            ('/', '`') => Some(Self::DishAntenna),
            ('/', 'a') => Some(Self::Ambulance),
            ('/', 'b') => Some(Self::Bike),
            ('/', 'c') => Some(Self::IncidentCommandPost),
            ('/', 'd') => Some(Self::FireDept),
            ('/', 'e') => Some(Self::HorseEquestrian),
            ('/', 'f') => Some(Self::FireTruck),
            ('/', 'g') => Some(Self::Glider),
            ('/', 'h') => Some(Self::Hospital),
            ('/', 'i') => Some(Self::IotaIslandsOnTheAir),
            ('/', 'j') => Some(Self::Jeep),
            ('/', 'k') => Some(Self::Truck),
            ('/', 'l') => Some(Self::Laptop),
            ('/', 'm') => Some(Self::MicERepeater),
            ('/', 'n') => Some(Self::NodeBlackBullseye),
            ('/', 'o') => Some(Self::Eoc),
            ('/', 'p') => Some(Self::Paraglider),
            ('/', 'q') => Some(Self::GridSqShownAbove128m),
            ('/', 'r') => Some(Self::Repeater),
            ('/', 's') => Some(Self::ShipPwrBoat),
            ('/', 't') => Some(Self::TruckStop),
            ('/', 'u') => Some(Self::Truck18Wheeler),
            ('/', 'v') => Some(Self::Van),
            ('/', 'w') => Some(Self::WaterStation),
            ('/', 'x') => Some(Self::XAprsUnix),
            ('/', 'y') => Some(Self::YagiAtQth),
            ('/', 'z') => Some(Self::TbdZ),
            ('/', '{') => None, // Empty in reference
            ('/', '|') => Some(Self::TncStreamSwitch1),
            ('/', '}') => None, // Empty in reference
            ('/', '~') => Some(Self::TncStreamSwitch2),

            // Alternate table (\)
            ('\\', '!') => Some(Self::EmergencyAndOverlays),
            ('\\', '"') => Some(Self::ReservedAlt),
            ('\\', '#') => Some(Self::OverlayDigiGreenStar),
            ('\\', '$') => Some(Self::BankAtmGreenBox),
            ('\\', '%') => Some(Self::PowerPlantWithOverlay),
            ('\\', '&') => Some(Self::IgateRxTx1hop2hop),
            ('\\', '\'') => Some(Self::CrashIncidentSites),
            ('\\', '(') => Some(Self::CloudyOtherClouds),
            ('\\', ')') => Some(Self::FirenetMeoModisEarthObs),
            ('\\', '*') => Some(Self::AvailSnowMoved),
            ('\\', '+') => Some(Self::Church),
            ('\\', ',') => Some(Self::GirlScouts),
            ('\\', '-') => Some(Self::HouseHfOpPresent),
            ('\\', '.') => Some(Self::AmbiguousBigQuestionMark),
            ('\\', '/') => Some(Self::WaypointDestination),
            ('\\', '0') => Some(Self::CircleIrlpEcholinkWires),
            ('\\', '1') => Some(Self::Avail1),
            ('\\', '2') => Some(Self::Avail2),
            ('\\', '3') => Some(Self::Avail3),
            ('\\', '4') => Some(Self::Avail4),
            ('\\', '5') => Some(Self::Avail5),
            ('\\', '6') => Some(Self::Avail6),
            ('\\', '7') => Some(Self::Avail7),
            ('\\', '8') => Some(Self::Network80211OrOther),
            ('\\', '9') => Some(Self::GasStationBluePump),
            ('\\', ':') => Some(Self::AvailHail),
            ('\\', ';') => Some(Self::ParkPicnicOverlayEvents),
            ('\\', '<') => Some(Self::AdvisoryOneWxFlag),
            ('\\', '=') => Some(Self::AvailSymbolOverlayGroup),
            ('\\', '>') => Some(Self::OverlayedCarsVehicles),
            ('\\', '?') => Some(Self::InfoKioskBlueBox),
            ('\\', '@') => Some(Self::HuricanetropStorm),
            ('\\', 'A') => Some(Self::OverlayBoxDtmfRfidXo),
            ('\\', 'B') => Some(Self::AvailBlowingSnow),
            ('\\', 'C') => Some(Self::CoastGuard),
            ('\\', 'D') => Some(Self::DepotsAndDrizzle),
            ('\\', 'E') => Some(Self::SmokeAndOtherVisCodes),
            ('\\', 'F') => Some(Self::AvailFreezingRain),
            ('\\', 'G') => Some(Self::AvailSnowShower),
            ('\\', 'H') => Some(Self::HazeAndOverlayHazards),
            ('\\', 'I') => Some(Self::RainShower),
            ('\\', 'J') => Some(Self::AvailLightning),
            ('\\', 'K') => Some(Self::KenwoodHt),
            ('\\', 'L') => Some(Self::Lighthouse),
            ('\\', 'M') => Some(Self::MarsArmyNavyAf),
            ('\\', 'N') => Some(Self::NavigationBuoy),
            ('\\', 'O') => Some(Self::OverlayBalloonRocket),
            ('\\', 'P') => Some(Self::Parking),
            ('\\', 'Q') => Some(Self::Quake),
            ('\\', 'R') => Some(Self::Restaurant),
            ('\\', 'S') => Some(Self::SatellitePacsat),
            ('\\', 'T') => Some(Self::Thunderstorm),
            ('\\', 'U') => Some(Self::Sunny),
            ('\\', 'V') => Some(Self::VortacNavAid),
            ('\\', 'W') => Some(Self::NwsSiteWithOptions),
            ('\\', 'X') => Some(Self::PharmacyRxApothicary),
            ('\\', 'Y') => Some(Self::RadiosAndDevices),
            ('\\', 'Z') => Some(Self::AvailZ),
            ('\\', '[') => Some(Self::WCloudAndHumansOverlay),
            ('\\', '\\') => Some(Self::NewOverlayableGpsSymbol),
            ('\\', ']') => Some(Self::AvailBackslash),
            ('\\', '^') => Some(Self::OtherAircraftOverlays),
            ('\\', '_') => Some(Self::WxSiteGreenDigi),
            ('\\', '`') => Some(Self::RainAllTypesWithOverlay),
            ('\\', 'a') => Some(Self::ArrlAresWinlinkDstar),
            ('\\', 'b') => Some(Self::AvailBlowingDustSand),
            ('\\', 'c') => Some(Self::CdTriangleRacesSatern),
            ('\\', 'd') => Some(Self::DxSpotByCallsign),
            ('\\', 'e') => Some(Self::SleetAndFutureOverlays),
            ('\\', 'f') => Some(Self::FunnelCloud),
            ('\\', 'g') => Some(Self::GaleFlags),
            ('\\', 'h') => Some(Self::StoreOrHamfest),
            ('\\', 'i') => Some(Self::BoxOrPointsOfInterest),
            ('\\', 'j') => Some(Self::WorkZoneSteamShovel),
            ('\\', 'k') => Some(Self::SpecialVehicleSuvAtv4x4),
            ('\\', 'l') => Some(Self::AreasBoxCircles),
            ('\\', 'm') => Some(Self::ValueSign3DigitDisplay),
            ('\\', 'n') => Some(Self::OverlayTriangle),
            ('\\', 'o') => Some(Self::SmallCircle),
            ('\\', 'p') => Some(Self::AvailPartlyCloudy),
            ('\\', 'q') => Some(Self::AvailQ),
            ('\\', 'r') => Some(Self::Restrooms),
            ('\\', 's') => Some(Self::OverlayShipBoats),
            ('\\', 't') => Some(Self::Tornado),
            ('\\', 'u') => Some(Self::OverlayedTruck),
            ('\\', 'v') => Some(Self::OverlayedVan),
            ('\\', 'w') => Some(Self::FloodingAvalanchesSlides),
            ('\\', 'x') => Some(Self::WreckOrObstruction),
            ('\\', 'y') => Some(Self::Skywarn),
            ('\\', 'z') => Some(Self::OverlayedShelter),
            ('\\', '{') => Some(Self::AvailFog),
            ('\\', '|') => Some(Self::TncStreamSwitchAlt1),
            ('\\', '}') => Some(Self::AvailMaybe),
            ('\\', '~') => Some(Self::TncStreamSwitchAlt2),

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_station_symbol() {
        let symbol = AprsSymbol::parse('/', '_');
        assert_eq!(symbol, Some(AprsSymbol::WeatherStation));
    }

    #[test]
    fn test_emergency_symbol() {
        let symbol = AprsSymbol::parse('\\', '!');
        assert_eq!(symbol, Some(AprsSymbol::EmergencyAndOverlays));
    }

    #[test]
    fn test_invalid_symbol() {
        let symbol = AprsSymbol::parse('/', '\x7f');
        assert_eq!(symbol, None);
    }

    #[test]
    fn test_invalid_table() {
        let symbol = AprsSymbol::parse('?', '_');
        assert_eq!(symbol, None);
    }

    #[test]
    fn test_glider_symbol() {
        let symbol = AprsSymbol::parse('/', 'g');
        assert_eq!(symbol, Some(AprsSymbol::Glider));
    }

    #[test]
    fn test_paraglider_symbol() {
        let symbol = AprsSymbol::parse('/', 'p');
        assert_eq!(symbol, Some(AprsSymbol::Paraglider));
    }

    #[test]
    fn test_balloon_symbol() {
        let symbol = AprsSymbol::parse('\\', 'O');
        assert_eq!(symbol, Some(AprsSymbol::OverlayBalloonRocket));
    }

    #[test]
    fn test_ground_wx_station_symbol() {
        let symbol = AprsSymbol::parse('/', '_');
        assert_eq!(symbol, Some(AprsSymbol::WeatherStation));
    }

    #[test]
    fn test_small_aircraft_symbol() {
        let symbol = AprsSymbol::parse('/', '\'');
        assert_eq!(symbol, Some(AprsSymbol::SmallAircraft));
    }

    #[test]
    fn test_large_aircraft_symbol() {
        let symbol = AprsSymbol::parse('/', '^');
        assert_eq!(symbol, Some(AprsSymbol::LargeAircraft));
    }

    #[test]
    fn test_node_symbol() {
        let symbol = AprsSymbol::parse('/', 'n');
        assert_eq!(symbol, Some(AprsSymbol::NodeBlackBullseye));
    }
}
