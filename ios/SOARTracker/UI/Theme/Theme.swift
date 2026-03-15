import SwiftUI

// MARK: - Colors (matching Android theme)

extension Color {
    // Primary
    static let soarPrimary = Color(light: .init(red: 0.086, green: 0.396, blue: 0.753),
                                   dark: .init(red: 0.565, green: 0.792, blue: 0.976))

    // Status colors
    static let soarGreen = Color(red: 0.298, green: 0.686, blue: 0.314) // #4CAF50
    static let soarRed = Color(red: 0.957, green: 0.263, blue: 0.212)   // #F44336
    static let soarOrange = Color(red: 1.0, green: 0.596, blue: 0.0)    // #FF9800

    // Surface / background helpers
    static let cardBackground = Color(.secondarySystemGroupedBackground)
    static let screenBackground = Color(.systemGroupedBackground)
}

// MARK: - Adaptive light/dark color initializer

extension Color {
    init(light: Color, dark: Color) {
        self.init(uiColor: UIColor { traits in
            traits.userInterfaceStyle == .dark ? UIColor(dark) : UIColor(light)
        })
    }
}

// MARK: - Flight State Styling

extension String {
    var flightStateColor: Color {
        switch self.lowercased() {
        case "active": return .soarGreen
        case "stale": return .soarOrange
        default: return .soarRed
        }
    }
}

// MARK: - Formatting Helpers

enum Formatters {
    /// Format duration in seconds to a readable string (H:MM:SS or M:SS).
    static func duration(_ seconds: Int) -> String {
        let h = seconds / 3600
        let m = (seconds % 3600) / 60
        let s = seconds % 60
        if h > 0 {
            return String(format: "%d:%02d:%02d", h, m, s)
        }
        return String(format: "%d:%02d", m, s)
    }

    /// Format an ISO 8601 timestamp to local time (HH:mm:ss).
    static func localTime(from iso: String) -> String? {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        if let date = formatter.date(from: iso) {
            let display = DateFormatter()
            display.timeStyle = .medium
            display.dateStyle = .none
            return display.string(from: date)
        }
        // Try without fractional seconds
        formatter.formatOptions = [.withInternetDateTime]
        if let date = formatter.date(from: iso) {
            let display = DateFormatter()
            display.timeStyle = .medium
            display.dateStyle = .none
            return display.string(from: date)
        }
        return nil
    }

    /// Format meters to nautical miles.
    static func nauticalMiles(meters: Double) -> String {
        let nm = meters / 1852.0
        return String(format: "%.1f nm", nm)
    }

    /// Format hPa to inHg.
    static func inHg(hPa: Double) -> String {
        let inHg = hPa / 33.8639
        return String(format: "%.2f", inHg)
    }

    /// Calculate barometric altitude from pressure ratio.
    /// Formula: 145366.45 * (1 - (P / P0) ^ 0.190284) in feet.
    static func barometricAltitude(pressureHpa: Double, groundPressureHpa: Double) -> Double {
        return 145366.45 * (1.0 - pow(pressureHpa / groundPressureHpa, 0.190284))
    }

    /// Format meters to feet.
    static func metersToFeet(_ meters: Double) -> Double {
        return meters * 3.28084
    }

    /// Bearing from point 1 to point 2 in degrees (0-360).
    static func bearing(lat1: Double, lon1: Double, lat2: Double, lon2: Double) -> Double {
        let lat1Rad = lat1 * .pi / 180.0
        let lat2Rad = lat2 * .pi / 180.0
        let dLon = (lon2 - lon1) * .pi / 180.0

        let y = sin(dLon) * cos(lat2Rad)
        let x = cos(lat1Rad) * sin(lat2Rad) - sin(lat1Rad) * cos(lat2Rad) * cos(dLon)
        var bearing = atan2(y, x) * 180.0 / .pi
        bearing = (bearing + 360.0).truncatingRemainder(dividingBy: 360.0)
        return bearing
    }

    /// Time ago string (e.g., "42s ago").
    static func timeAgo(from date: Date) -> String {
        let seconds = Int(Date().timeIntervalSince(date))
        if seconds < 60 {
            return "\(seconds)s ago"
        } else if seconds < 3600 {
            return "\(seconds / 60)m ago"
        }
        return "\(seconds / 3600)h ago"
    }

    /// Staleness from ISO timestamp.
    static func staleness(from iso: String) -> String? {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        var date = formatter.date(from: iso)
        if date == nil {
            formatter.formatOptions = [.withInternetDateTime]
            date = formatter.date(from: iso)
        }
        guard let d = date else { return nil }
        return timeAgo(from: d)
    }

    /// Cardinal direction for latitude.
    static func latCardinal(_ lat: Double) -> String {
        return lat >= 0 ? "N" : "S"
    }

    /// Cardinal direction for longitude.
    static func lonCardinal(_ lon: Double) -> String {
        return lon >= 0 ? "E" : "W"
    }
}
