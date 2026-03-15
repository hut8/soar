import SwiftUI

struct FlightStatusCard: View {
    let response: TrackerFixResponse

    var body: some View {
        VStack(spacing: 0) {
            // Aircraft section
            if let aircraft = response.matchedAircraft {
                aircraftSection(aircraft)
                if response.flight != nil {
                    Divider().padding(.horizontal, 16)
                }
            }

            // Flight section
            if let flight = response.flight {
                flightSection(flight)
            }
        }
        .background(Color.cardBackground)
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }

    // MARK: - Aircraft Section

    @ViewBuilder
    private func aircraftSection(_ aircraft: TrackerAircraftInfo) -> some View {
        VStack(spacing: 4) {
            HStack {
                Image(systemName: "airplane")
                    .foregroundStyle(.tint)

                Text("Aircraft")
                    .font(.headline)

                Spacer()
            }

            if let reg = aircraft.registration {
                InfoRow(label: "Registration", value: reg)
            }
            if let model = aircraft.aircraftModel {
                InfoRow(label: "Model", value: model)
            }
            if let comp = aircraft.competitionNumber {
                InfoRow(label: "Competition #", value: comp)
            }
            if let dist = aircraft.distanceMeters {
                InfoRow(label: "Distance", value: Formatters.nauticalMiles(meters: dist))
            }
        }
        .padding(16)
    }

    // MARK: - Flight Section

    @ViewBuilder
    private func flightSection(_ flight: TrackerFlightInfo) -> some View {
        VStack(spacing: 4) {
            HStack {
                Image(systemName: "chart.line.uptrend.xyaxis")
                    .foregroundStyle(.tint)

                Text("Flight")
                    .font(.headline)

                Spacer()

                if let state = flight.state {
                    FlightStateBadge(state: state)
                }
            }

            if let takeoff = flight.takeoffTime, let local = Formatters.localTime(from: takeoff) {
                InfoRow(label: "Takeoff", value: local)
            }
            if let duration = flight.durationSeconds {
                InfoRow(label: "Duration", value: Formatters.duration(duration))
            }
            if let airport = flight.departureAirport {
                InfoRow(label: "Departure", value: airport)
            }
            if let towRelease = flight.towReleaseTime, let local = Formatters.localTime(from: towRelease) {
                InfoRow(label: "Tow Release", value: local)
            }
            if let towAlt = flight.towReleaseAltitudeMslFt {
                InfoRow(label: "Release Alt", value: "\(Int(towAlt)) ft MSL")
            }
        }
        .padding(16)
    }
}

// MARK: - Flight State Badge

private struct FlightStateBadge: View {
    let state: String

    var body: some View {
        HStack(spacing: 4) {
            Circle()
                .fill(state.flightStateColor)
                .frame(width: 8, height: 8)
            Text(state.capitalized)
                .font(.caption)
                .fontWeight(.medium)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(
            Capsule()
                .fill(state.flightStateColor.opacity(0.15))
        )
    }
}

// MARK: - Info Row

private struct InfoRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack {
            Text(label)
                .font(.footnote)
                .foregroundStyle(.secondary)
            Spacer()
            Text(value)
                .font(.footnote)
                .fontWeight(.medium)
        }
    }
}
