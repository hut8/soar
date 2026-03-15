import Foundation

// MARK: - Authentication

struct LoginRequest: Codable {
    let email: String
    let password: String
}

struct LoginResponse: Codable {
    let token: String
    let user: UserInfo
}

struct UserInfo: Codable {
    let id: String
    let firstName: String?
    let lastName: String?
    let email: String
}

struct PasswordResetRequest: Codable {
    let email: String
}

// MARK: - Tracker

struct TrackerFixRequest: Codable {
    let latitude: Double
    let longitude: Double
    let heading: Double?
    let altitudeMeters: Double?
    let speedMps: Double?
    let accuracyMeters: Double?
    let pressureHpa: Double?
    let accelX: Double?
    let accelY: Double?
    let accelZ: Double?
    let verticalSpeedMps: Double?
}

struct TrackerFixResponse: Codable {
    let matchedAircraft: TrackerAircraftInfo?
    let flight: TrackerFlightInfo?
    let nearbyAircraft: [NearbyAircraftInfo]?
    let groundElevationMeters: Double?
    let altitudeAglFeet: Double?
    let magneticDeclinationDegrees: Double?
}

struct TrackerAircraftInfo: Codable {
    let id: String?
    let registration: String?
    let aircraftModel: String?
    let competitionNumber: String?
    let distanceMeters: Double?
}

struct TrackerFlightInfo: Codable {
    let id: String?
    let state: String?
    let takeoffTime: String?
    let departureAirport: String?
    let durationSeconds: Int?
    let towReleaseTime: String?
    let towReleaseAltitudeMslFt: Double?
}

struct NearbyAircraftInfo: Codable, Identifiable {
    let id: String?
    let registration: String?
    let aircraftModel: String?
    let competitionNumber: String?
    let latitude: Double?
    let longitude: Double?
    let altitudeFeet: Double?
    let groundSpeedKnots: Double?
    let climbFpm: Double?
    let trackDegrees: Double?
    let distanceMeters: Double?
    let lastFixAt: String?

    var stableId: String {
        id ?? UUID().uuidString
    }
}
