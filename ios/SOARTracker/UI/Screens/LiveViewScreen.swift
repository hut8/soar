import SwiftUI

struct LiveViewScreen: View {
    @EnvironmentObject var trackingService: TrackingService

    var body: some View {
        NavigationStack {
            Group {
                if !trackingService.isTracking {
                    emptyState(
                        icon: "location.slash",
                        message: "Start tracking to see nearby aircraft"
                    )
                } else if let nearby = trackingService.latestResponse?.nearbyAircraft, !nearby.isEmpty {
                    aircraftList(nearby)
                } else {
                    emptyState(
                        icon: "airplane",
                        message: "No aircraft nearby"
                    )
                }
            }
            .navigationTitle("Live View")
            .background(Color.screenBackground)
        }
    }

    private func emptyState(icon: String, message: String) -> some View {
        VStack(spacing: 16) {
            Image(systemName: icon)
                .font(.system(size: 48))
                .foregroundStyle(.secondary)
            Text(message)
                .font(.headline)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func aircraftList(_ aircraft: [NearbyAircraftInfo]) -> some View {
        ScrollView {
            LazyVStack(spacing: 12) {
                ForEach(aircraft, id: \.stableId) { ac in
                    LiveAircraftCard(aircraft: ac)
                }
            }
            .padding()
        }
    }
}

// MARK: - Live Aircraft Card

private struct LiveAircraftCard: View {
    let aircraft: NearbyAircraftInfo

    var body: some View {
        VStack(spacing: 0) {
            // Header: Name + bearing + distance
            HStack {
                Text(displayName)
                    .font(.headline)

                Spacer()

                if let bearing = computedBearing {
                    Image(systemName: "location.north.fill")
                        .rotationEffect(.degrees(bearing))
                        .foregroundStyle(.secondary)
                }

                if let dist = aircraft.distanceMeters {
                    Text(Formatters.nauticalMiles(meters: dist))
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)

            Divider()

            // Details grid
            HStack(spacing: 0) {
                DetailColumn(label: "Alt", value: altitudeText)
                DetailColumn(label: "GS", value: groundSpeedText)
                DetailColumn(label: "Climb", value: climbText, color: climbColor)
                DetailColumn(label: "Track", value: trackText)
            }
            .padding(.vertical, 10)

            // Staleness
            if let lastFix = aircraft.lastFixAt, let staleness = Formatters.staleness(from: lastFix) {
                HStack {
                    Spacer()
                    Text(staleness)
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }
                .padding(.horizontal, 16)
                .padding(.bottom, 8)
            }
        }
        .background(Color.cardBackground)
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }

    private var displayName: String {
        aircraft.registration ?? aircraft.aircraftModel ?? aircraft.competitionNumber ?? "Unknown"
    }

    private var computedBearing: Double? {
        // We don't have our own position here, so we can't compute bearing
        // The Android version also doesn't compute it in LiveView (only compact list)
        aircraft.trackDegrees
    }

    private var altitudeText: String {
        if let alt = aircraft.altitudeFeet {
            return "\(Int(alt)) ft"
        }
        return "--"
    }

    private var groundSpeedText: String {
        if let gs = aircraft.groundSpeedKnots {
            return "\(Int(gs)) kt"
        }
        return "--"
    }

    private var climbText: String {
        if let climb = aircraft.climbFpm {
            let arrow = climb > 0 ? "\u{25B2}" : (climb < 0 ? "\u{25BC}" : "")
            return "\(arrow)\(Int(abs(climb))) fpm"
        }
        return "--"
    }

    private var climbColor: Color {
        guard let climb = aircraft.climbFpm else { return .primary }
        if climb > 50 { return .soarGreen }
        if climb < -50 { return .soarRed }
        return .primary
    }

    private var trackText: String {
        if let track = aircraft.trackDegrees {
            return "\(Int(track))\u{00B0}"
        }
        return "--"
    }
}

// MARK: - Detail Column

private struct DetailColumn: View {
    let label: String
    let value: String
    var color: Color = .primary

    var body: some View {
        VStack(spacing: 2) {
            Text(label)
                .font(.caption2)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.caption)
                .fontWeight(.medium)
                .foregroundStyle(color)
        }
        .frame(maxWidth: .infinity)
    }
}
