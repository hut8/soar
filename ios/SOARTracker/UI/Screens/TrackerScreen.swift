import CoreLocation
import SwiftUI

struct TrackerScreen: View {
    @EnvironmentObject var trackingService: TrackingService
    @EnvironmentObject var authState: ObservableAuthState

    @State private var showLogoutConfirm = false
    @State private var showAltimeterConfirm = false

    var body: some View {
        NavigationStack {
            ZStack(alignment: .bottomTrailing) {
                ScrollView {
                    VStack(spacing: 16) {
                        // Error card
                        if let error = trackingService.lastError {
                            ErrorCard(message: error)
                        }

                        // Flight status
                        if let response = trackingService.latestResponse {
                            FlightStatusCard(response: response)
                        }

                        // Sensor display
                        SensorDisplay(
                            sensorData: trackingService.sensorCollector.sensorData,
                            response: trackingService.latestResponse,
                            magneticHeading: trackingService.magneticHeading,
                            groundPressureHpa: trackingService.groundPressureHpa,
                            gnssAltitudeMeters: trackingService.lastGNSSAltitudeMeters,
                            latitude: trackingService.lastLatitude,
                            longitude: trackingService.lastLongitude,
                            gpsTrack: trackingService.lastGPSTrack,
                            verticalSpeedMps: trackingService.lastVerticalSpeedMps,
                            onSetAltimeter: handleSetAltimeter
                        )

                        // Nearby aircraft (compact)
                        if let nearby = trackingService.latestResponse?.nearbyAircraft, !nearby.isEmpty {
                            NearbyAircraftList(aircraft: nearby, compact: true)
                        }

                        // Open website button
                        Button(action: openWebsite) {
                            Label("Open glider.flights", systemImage: "safari")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.bordered)
                        .padding(.bottom, 80)
                    }
                    .padding()
                }
                .background(Color.screenBackground)

                // Tracking FAB
                TrackingFAB(
                    isTracking: trackingService.isTracking,
                    onToggle: toggleTracking
                )
                .padding(20)
            }
            .navigationTitle("SOAR Tracker")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    HStack(spacing: 12) {
                        ConnectionStatus(
                            isTracking: trackingService.isTracking,
                            lastUpdate: trackingService.lastUpdateTime
                        )

                        Button(action: { showLogoutConfirm = true }) {
                            Image(systemName: "rectangle.portrait.and.arrow.right")
                        }
                    }
                }
            }
            .alert("Log Out", isPresented: $showLogoutConfirm) {
                Button("Log Out", role: .destructive) {
                    trackingService.stopTracking()
                    authState.logout()
                }
                Button("Cancel", role: .cancel) {}
            } message: {
                Text("Are you sure you want to log out?")
            }
            .alert("Set Altimeter", isPresented: $showAltimeterConfirm) {
                Button("Set") {
                    trackingService.groundPressureHpa = trackingService.sensorCollector.sensorData.pressureHpa
                }
                Button("Cancel", role: .cancel) {}
            } message: {
                Text("Set current pressure as ground reference? This should only be done on the ground.")
            }
        }
    }

    private func toggleTracking() {
        if trackingService.isTracking {
            trackingService.stopTracking()
        } else {
            let status = trackingService.authorizationStatus
            if status == .notDetermined {
                trackingService.requestLocationPermission()
                return
            }
            guard status == .authorizedWhenInUse || status == .authorizedAlways else {
                trackingService.lastError = "Location permission denied. Please enable in Settings."
                return
            }
            guard let api = authState.createAuthenticatedAPI() else {
                trackingService.lastError = "Not authenticated. Please log in again."
                return
            }
            trackingService.startTracking(api: api)
        }
    }

    private func handleSetAltimeter() {
        // If a flight is active, confirm first
        if trackingService.latestResponse?.flight?.state?.lowercased() == "active" {
            showAltimeterConfirm = true
        } else {
            trackingService.groundPressureHpa = trackingService.sensorCollector.sensorData.pressureHpa
        }
    }

    private func openWebsite() {
        let url = URL(string: TokenManager.shared.serverURL)!
        UIApplication.shared.open(url)
    }
}

// MARK: - Connection Status Indicator

private struct ConnectionStatus: View {
    let isTracking: Bool
    let lastUpdate: Date?

    @State private var now = Date()
    let timer = Timer.publish(every: 1, on: .main, in: .common).autoconnect()

    var body: some View {
        HStack(spacing: 4) {
            Circle()
                .fill(statusColor)
                .frame(width: 8, height: 8)

            if let lastUpdate = lastUpdate {
                Text(Formatters.timeAgo(from: lastUpdate))
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
        .onReceive(timer) { now = $0 }
    }

    private var statusColor: Color {
        guard isTracking, let lastUpdate = lastUpdate else {
            return isTracking ? .orange : .gray
        }
        let age = now.timeIntervalSince(lastUpdate)
        return age < 10 ? .soarGreen : (age < 30 ? .soarOrange : .soarRed)
    }
}

// MARK: - Tracking FAB

private struct TrackingFAB: View {
    let isTracking: Bool
    let onToggle: () -> Void

    var body: some View {
        Button(action: onToggle) {
            Image(systemName: isTracking ? "stop.fill" : "play.fill")
                .font(.title2)
                .foregroundColor(.white)
                .frame(width: 56, height: 56)
                .background(isTracking ? Color.soarRed : Color.soarGreen)
                .clipShape(Circle())
                .shadow(radius: 4)
        }
    }
}

// MARK: - Error Card

private struct ErrorCard: View {
    let message: String

    var body: some View {
        HStack {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundStyle(.red)
            Text(message)
                .font(.footnote)
                .foregroundStyle(.red)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 8)
                .fill(Color.red.opacity(0.1))
        )
    }
}
