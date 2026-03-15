import SwiftUI

struct NearbyAircraftList: View {
    let aircraft: [NearbyAircraftInfo]
    var compact: Bool = false

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Image(systemName: "airplane")
                    .foregroundStyle(.tint)
                Text("Nearby Aircraft")
                    .font(.headline)
                Spacer()
                Text("\(aircraft.count)")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
            .padding(.horizontal, 16)
            .padding(.top, 12)
            .padding(.bottom, 8)

            Divider().padding(.horizontal, 16)

            // Aircraft rows
            let displayed = compact ? Array(aircraft.prefix(5)) : aircraft
            ForEach(displayed, id: \.stableId) { ac in
                NearbyAircraftRow(aircraft: ac)
                if ac.stableId != displayed.last?.stableId {
                    Divider().padding(.horizontal, 16)
                }
            }
        }
        .background(Color.cardBackground)
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }
}

// MARK: - Nearby Aircraft Row

private struct NearbyAircraftRow: View {
    let aircraft: NearbyAircraftInfo

    var body: some View {
        HStack(spacing: 12) {
            // Registration / model
            VStack(alignment: .leading, spacing: 2) {
                Text(displayName)
                    .font(.subheadline)
                    .fontWeight(.medium)
                    .lineLimit(1)

                if let alt = aircraft.altitudeFeet {
                    Text("\(Int(alt)) ft")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            Spacer()

            // Bearing arrow (using track as proxy for bearing from our position)
            if let track = aircraft.trackDegrees {
                Image(systemName: "location.north.fill")
                    .font(.caption)
                    .rotationEffect(.degrees(track))
                    .foregroundStyle(.secondary)
            }

            // Distance
            if let dist = aircraft.distanceMeters {
                Text(Formatters.nauticalMiles(meters: dist))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .frame(minWidth: 48, alignment: .trailing)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
    }

    private var displayName: String {
        aircraft.registration ?? aircraft.aircraftModel ?? aircraft.competitionNumber ?? "Unknown"
    }
}
